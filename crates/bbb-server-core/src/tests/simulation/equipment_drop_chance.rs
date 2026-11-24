use super::*;

use crate::entity::{EntityData, GeneralLivingData};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID;

fn drop_chance_record(chance: f32) -> Vec<NbtSummaryEntry> {
    vec![NbtSummaryEntry {
        name: "drop_chances".into(),
        value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "mainhand".into(),
            value: NbtSummaryValue::Float(chance.to_bits()),
        }]),
    }]
}

fn equipment_pack_config(entries: serde_json::Value) -> ServerWorldConfig {
    let dir = std::env::temp_dir().join(format!("bbb-equipment-looting-{}", Uuid::new_v4()));
    let pack = dir.join("datapacks/test");
    std::fs::create_dir_all(pack.join("data/minecraft/enchantment")).unwrap();
    std::fs::write(
        pack.join("pack.mcmeta"),
        "{\"pack\":{\"description\":\"equipment test\",\"pack_format\":1000,\"supported_formats\":[0,10000]}}",
    ).unwrap();
    let index = crate::enchantment_facts::builtin_enchantment_registry()
        .iter()
        .position(|id| id == "minecraft:looting")
        .unwrap();
    let mut document =
        serde_json::to_value(&crate::enchantment_facts::builtin_enchantment_documents()[index])
            .unwrap()
            .clone();
    document["effects"]["minecraft:equipment_drops"] = entries;
    std::fs::write(
        pack.join("data/minecraft/enchantment/looting.json"),
        serde_json::to_vec(&document).unwrap(),
    )
    .unwrap();
    let startup = bbb_server_datapack::startup::DatapackStartup::load(
        &bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: Some(dir.join("datapacks")),
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        },
    )
    .unwrap();
    std::fs::remove_dir_all(dir).unwrap();
    let mut config = operator_config(&[("alice", 1)]);
    config.datapacks = Some(Arc::new(startup));
    config
}

fn looting_pack_config() -> ServerWorldConfig {
    let index = crate::enchantment_facts::builtin_enchantment_registry()
        .iter()
        .position(|id| id == "minecraft:looting")
        .unwrap();
    let mut entries =
        serde_json::to_value(&crate::enchantment_facts::builtin_enchantment_documents()[index])
            .unwrap()["effects"]["minecraft:equipment_drops"]
            .clone();
    entries[0]["effect"] = serde_json::json!({"type": "minecraft:add", "value": 2.0});
    equipment_pack_config(entries)
}

fn setup_victim(world: &mut ServerWorld, chance: f32) -> i32 {
    setup_victim_with_record(world, &drop_chance_record(chance))
}

fn setup_victim_with_record(world: &mut ServerWorld, record: &[NbtSummaryEntry]) -> i32 {
    let position = Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    let id = world
        .spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            position,
            EntityData::GeneralLiving(
                GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID).unwrap(),
            ),
        )
        .0;
    world.load_stored_entity_record_for_test(Java26BuiltinDimension::Overworld, id, record);
    let victim = world
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, id)
        .unwrap();
    victim.set_pos(position);
    victim.living_mut().unwrap().set_item_slot(
        EquipmentSlot::MainHand,
        Some(crate::entity::item::plain_stack("minecraft:diamond_sword", 1).unwrap()),
    );
    crate::entity::living::set_health(victim, 0.1);
    id
}

fn equip_player_looting(world: &mut ServerWorld) {
    let registry = world
        .datapacks()
        .unwrap()
        .registries
        .registry("minecraft:enchantment")
        .unwrap();
    let looting = registry
        .elements
        .iter()
        .position(|entry| entry.id == "minecraft:looting")
        .unwrap();
    let mut sword = ItemStackSummary {
        item_id: Some(
            bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:diamond_sword")
                .unwrap()
                .id,
        ),
        count: 1,
        ..ItemStackSummary::empty()
    };
    sword
        .component_patch
        .enchantments
        .push(bbb_protocol::packets::ItemEnchantmentSummary {
            holder_id: looting as i32,
            level: 3,
        });
    sword
        .component_patch
        .added_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID);
    world.set_inventory_slot_for_test(ConnectionId(1), 0, sword);
}

#[test]
fn player_mainhand_looting_from_loaded_generation_changes_real_equipment_drop() {
    for (chance, expected) in [(0.0001, true), (0.0, false)] {
        let (sender, receiver, mut world) = helpers::joined_stand_world(
            looting_pack_config(),
            Vec3d {
                x: 2.5,
                y: 64.0,
                z: 0.5,
            },
        );
        equip_player_looting(&mut world);
        let victim = setup_victim(&mut world, chance);
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::Attack(AttackEntity { entity_id: victim }),
        );
        assert_eq!(
            world
                .stored_items_for_test(Java26BuiltinDimension::Overworld)
                .iter()
                .any(|item| item.2 == "minecraft:diamond_sword"),
            expected,
            "original chance {chance}"
        );
    }
}

#[test]
fn equipment_drop_conditions_read_the_named_world_clock() {
    let mut config = datapack_config();
    config.datapacks = equipment_pack_config(serde_json::json!([{
        "enchanted":"attacker", "effect":{"type":"minecraft:set","value":0},
        "requirements":{"condition":"minecraft:time_check","clock":"minecraft:the_end","value":107}
    }, {
        "enchanted":"attacker", "effect":{"type":"minecraft:set","value":0},
        "requirements":{"condition":"minecraft:weather_check","raining":true}
    }, {
        "enchanted":"attacker", "effect":{"type":"minecraft:set","value":0},
        "requirements":{"condition":"minecraft:all_of","terms":[
            {"condition":"minecraft:entity_properties","entity":"this","predicate":{
                "type":"minecraft:player","vehicle":{"type":"minecraft:oak_chest_boat",
                    "slots":{"container.*":{"count":{"min":1}}}}}},
            {"condition":"minecraft:entity_properties","entity":"attacker",
                "predicate":{"type":"minecraft:player"}}
        ]}
    }]))
    .datapacks;
    for (time, expected) in [(107, false), (108, true)] {
        let (sender, receiver) = mailbox();
        let mut world = spawn_pinned_world(config.clone());
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        set_paused_test_clocks(&sender, &receiver, &mut world, time, 0);
        stand_at(
            &sender,
            &receiver,
            &mut world,
            1,
            Vec3d {
                x: 2.5,
                y: 64.0,
                z: 0.5,
            },
        );
        let equip = console_command(&sender, &receiver, &mut world, 1,
            "item replace entity alice weapon.mainhand with diamond_sword[enchantments={looting:1}]");
        assert_eq!(
            super::super::entities::console_command_answer(&equip).0,
            Some(1)
        );
        let victim = setup_victim(&mut world, 1.0);
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::Attack(AttackEntity { entity_id: victim }),
        );
        assert!(world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, victim)
            .is_none_or(|entity| entity.living().unwrap().health() <= 0.0));
        assert_eq!(
            world
                .stored_items_for_test(Java26BuiltinDimension::Overworld)
                .iter()
                .any(|item| item.2 == "minecraft:diamond_sword"),
            expected,
            "clock {time}"
        );
        if time != 108 {
            continue;
        }
        for (index, raining) in [true, false].into_iter().enumerate() {
            let changed = console_command(
                &sender,
                &receiver,
                &mut world,
                10 + index as u64 * 2,
                if raining {
                    "weather rain 100000t"
                } else {
                    "weather clear 100000t"
                },
            );
            assert_eq!(
                super::super::entities::console_command_answer(&changed).0,
                Some(100000)
            );
            for _ in 0..200 {
                if world
                    .level_weather(Java26BuiltinDimension::Overworld)
                    .is_raining()
                    == raining
                {
                    break;
                }
                world.tick(receiver.drain());
            }
            assert_eq!(
                world
                    .level_weather(Java26BuiltinDimension::Overworld)
                    .is_raining(),
                raining
            );
            stand_at(
                &sender,
                &receiver,
                &mut world,
                1,
                Vec3d {
                    x: 2.5,
                    y: 64.0,
                    z: 0.5,
                },
            );
            let uuid = Uuid::from_u128(6301 + index as u128);
            let summoned = console_command(
                &sender, &receiver, &mut world, 11 + index as u64 * 2,
                &format!("summon minecraft:husk 0.5 64 0.5 {{UUID:[I;0,0,0,{}],Tags:['equipment_weather_{index}'],NoAI:1b,NoGravity:1b,Health:0.1f,equipment:{{mainhand:{{id:'minecraft:iron_axe',count:1}}}},drop_chances:{{mainhand:1f}}}}", 6301 + index),
            );
            assert_eq!(
                super::super::entities::console_command_answer(&summoned).0,
                Some(1)
            );
            let victim = world
                .live_entity_id_for_uuid_for_test(Java26BuiltinDimension::Overworld, uuid)
                .unwrap();
            let body = world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, victim)
                .unwrap();
            assert_eq!(body.uuid(), uuid);
            assert_eq!(
                body.entity_type_id(),
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HUSK_ID
            );
            let held = body
                .living()
                .unwrap()
                .item_by_slot(EquipmentSlot::MainHand)
                .unwrap();
            assert_eq!(held.item_resource_id(), Some("minecraft:iron_axe"));
            assert_eq!(held.count(), 1);
            play(
                &sender,
                &receiver,
                &mut world,
                1,
                PlayServerbound::Attack(AttackEntity { entity_id: victim }),
            );
            assert!(world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, victim)
                .is_none_or(|entity| entity.living().unwrap().health() <= 0.0));
            assert_eq!(
                world
                    .level_weather(Java26BuiltinDimension::Overworld)
                    .is_raining(),
                raining
            );
            assert_eq!(
                world
                    .stored_items_for_test(Java26BuiltinDimension::Overworld)
                    .iter()
                    .filter(|item| item.2 == "minecraft:iron_axe")
                    .map(|item| item.3)
                    .sum::<i32>(),
                if raining { 0 } else { 1 }
            );
        }
        assert!(world
            .datapacks()
            .unwrap()
            .loot
            .table("minecraft:chests/spawn_bonus_chest")
            .is_some());
        let survival = console_command(
            &sender,
            &receiver,
            &mut world,
            19,
            "gamemode survival alice",
        );
        assert_eq!(
            super::super::entities::console_command_answer(&survival).0,
            Some(1)
        );
        let dimension = Java26BuiltinDimension::Overworld;
        let boat_uuid = Uuid::from_u128(6401);
        let summoned = console_command(
            &sender, &receiver, &mut world, 20,
            "summon minecraft:oak_chest_boat 2.5 64 0.5 {UUID:[I;0,0,0,6401],NoGravity:1b,LootTable:'minecraft:chests/spawn_bonus_chest',LootTableSeed:37L}",
        );
        assert_eq!(
            super::super::entities::console_command_answer(&summoned).0,
            Some(1)
        );
        let boat = world
            .live_entity_id_for_uuid_for_test(dimension, boat_uuid)
            .unwrap();
        let body = world.stored_entity_for_test(dimension, boat).unwrap();
        assert_eq!(body.uuid(), boat_uuid);
        assert_eq!(
            body.entity_type_id(),
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID
        );
        assert!(crate::entity::item_slots::container_has_loot(body));
        for slot in 0..crate::entity::item_slots::container_size(body).unwrap() {
            assert!(crate::entity::item_slots::read(body, slot as i32)
                .unwrap()
                .is_some_and(|stack| stack.is_empty()));
        }
        let NbtSummaryValue::Compound(before) = world
            .stored_entity_command_data_for_test(dimension, boat)
            .unwrap()
        else {
            panic!("pending equipment vehicle")
        };
        assert!(before.iter().any(|entry| entry.name == "LootTable"
            && entry.value
                == NbtSummaryValue::String("minecraft:chests/spawn_bonus_chest".into())));
        assert!(
            before
                .iter()
                .any(|entry| entry.name == "LootTableSeed"
                    && entry.value == NbtSummaryValue::Long(37))
        );
        assert!(!before.iter().any(|entry| entry.name == "Items"));
        let mounted = console_command(
            &sender,
            &receiver,
            &mut world,
            21,
            &format!("ride alice mount {boat_uuid}"),
        );
        assert_eq!(
            super::super::entities::console_command_answer(&mounted).0,
            Some(1)
        );
        for id in mounted.events.iter().filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::PlayerPosition(update),
            } => Some(update.id),
            _ => None,
        }) {
            play(
                &sender,
                &receiver,
                &mut world,
                1,
                PlayServerbound::AcceptTeleportation { id },
            );
        }
        assert_eq!(
            world
                .external_player_vehicle_state_for_test(ConnectionId(1))
                .2,
            Some(boat)
        );
        let mut filled_items = None;
        for (index, expected_shovels) in [0, 1].into_iter().enumerate() {
            if index == 1 {
                let dismounted =
                    console_command(&sender, &receiver, &mut world, 23, "ride alice dismount");
                assert_eq!(
                    super::super::entities::console_command_answer(&dismounted).0,
                    Some(1)
                );
                for id in dismounted.events.iter().filter_map(|event| match event {
                    WorldEvent::Send {
                        connection_id: ConnectionId(1),
                        packet: ClientboundPlayPacket::PlayerPosition(update),
                    } => Some(update.id),
                    _ => None,
                }) {
                    play(
                        &sender,
                        &receiver,
                        &mut world,
                        1,
                        PlayServerbound::AcceptTeleportation { id },
                    );
                }
                assert_eq!(
                    world
                        .external_player_vehicle_state_for_test(ConnectionId(1))
                        .2,
                    None
                );
                stand_at(
                    &sender,
                    &receiver,
                    &mut world,
                    1,
                    Vec3d {
                        x: 2.5,
                        y: 64.0,
                        z: 0.5,
                    },
                );
            }
            let uuid = Uuid::from_u128(6402 + index as u128);
            let summoned = console_command(
                &sender, &receiver, &mut world, 22 + index as u64 * 2,
                &format!("summon minecraft:husk 0.5 64 0.5 {{UUID:[I;0,0,0,{}],NoAI:1b,NoGravity:1b,Health:0.1f,equipment:{{mainhand:{{id:'minecraft:iron_shovel',count:1}}}},drop_chances:{{mainhand:1f}}}}", 6402 + index),
            );
            assert_eq!(
                super::super::entities::console_command_answer(&summoned).0,
                Some(1)
            );
            let victim = world
                .live_entity_id_for_uuid_for_test(dimension, uuid)
                .unwrap();
            let body = world.stored_entity_for_test(dimension, victim).unwrap();
            assert_eq!(body.uuid(), uuid);
            assert_eq!(
                body.entity_type_id(),
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HUSK_ID
            );
            let held = body
                .living()
                .unwrap()
                .item_by_slot(EquipmentSlot::MainHand)
                .unwrap();
            assert_eq!(held.item_resource_id(), Some("minecraft:iron_shovel"));
            assert_eq!(held.count(), 1);
            play(
                &sender,
                &receiver,
                &mut world,
                1,
                PlayServerbound::Attack(AttackEntity { entity_id: victim }),
            );
            assert!(world
                .stored_entity_for_test(dimension, victim)
                .is_none_or(|entity| entity.living().unwrap().health() <= 0.0));
            assert_eq!(
                world
                    .external_player_vehicle_state_for_test(ConnectionId(1))
                    .2,
                (index == 0).then_some(boat)
            );
            assert_eq!(
                world
                    .stored_passengers_for_test(dimension, boat)
                    .contains(&world.player_entity_id_for_test(ConnectionId(1))),
                index == 0
            );
            assert_eq!(
                world
                    .stored_items_for_test(dimension)
                    .iter()
                    .filter(|item| item.2 == "minecraft:iron_shovel")
                    .map(|item| item.3)
                    .sum::<i32>(),
                expected_shovels
            );
            let body = world.stored_entity_for_test(dimension, boat).unwrap();
            assert_eq!(body.uuid(), boat_uuid);
            assert!(!crate::entity::item_slots::container_has_loot(body));
            assert!(
                (0..crate::entity::item_slots::container_size(body).unwrap()).any(|slot| {
                    crate::entity::item_slots::read(body, slot as i32)
                        .unwrap()
                        .is_some_and(|stack| !stack.is_empty())
                })
            );
            let NbtSummaryValue::Compound(after) = world
                .stored_entity_command_data_for_test(dimension, boat)
                .unwrap()
            else {
                panic!("unpacked equipment vehicle")
            };
            assert!(after.iter().any(|entry| entry.name == "Items"));
            let items = field(&after, "Items").clone();
            if let Some(filled) = &filled_items {
                assert_eq!(&items, filled);
            } else {
                filled_items = Some(items);
            }
            assert!(!after
                .iter()
                .any(|entry| entry.name == "LootTable" || entry.name == "LootTableSeed"));
            if index == 0 {
                assert!(!world.level_weather(dimension).is_raining());
            }
        }
    }
}

#[test]
fn a_nonliving_damage_source_keeps_the_original_preserved_chance() {
    let mut world = spawn_pinned_world(datapack_config());
    let position = Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    let boat = world
        .spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
            position,
            EntityData::Boat(crate::entity::BoatData::new()),
        )
        .0;
    let boat_uuid = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, boat)
        .unwrap()
        .uuid();
    let victim = setup_victim(&mut world, 2.0);
    let outcome = world
        .hurt_stored_entity_by_command_for_test(
            Java26BuiltinDimension::Overworld,
            victim,
            "minecraft:generic",
            40.0,
            Some(crate::commands::CommandDamageSourceEntity::Stored {
                entity_id: boat,
                uuid: Some(boat_uuid),
                position: Some(position),
                entity_resource_id: "minecraft:oak_boat".into(),
                is_mob: false,
                is_living_non_player: false,
                display_name: NbtSummaryValue::String("boat".into()),
            }),
        )
        .unwrap();
    assert!(outcome.applied && outcome.succeeded);
    assert!(world
        .stored_items_for_test(Java26BuiltinDimension::Overworld)
        .iter()
        .any(|item| item.2 == "minecraft:diamond_sword"));
}

#[test]
fn a_riding_killers_equipment_effect_reads_the_current_dying_vehicle_movement() {
    let entries = serde_json::json!([
        {"enchanted":"attacker", "effect":{"type":"minecraft:set","value":0}},
        {"enchanted":"attacker", "effect":{"type":"minecraft:set","value":1},
         "requirements":{"condition":"minecraft:all_of","terms":[
             {"condition":"minecraft:entity_properties","entity":"attacker",
              "predicate":{"movement":{"y":{"min":4,"max":6}},
                  "nbt":"{SelectedItem:{id:'minecraft:diamond_sword',count:1,components:{'minecraft:custom_data':{rider:1}}},RootVehicle:{Entity:{id:'minecraft:zombie'}}}"}},
             {"condition":"minecraft:environment_attribute_check",
              "attribute":"minecraft:gameplay/water_evaporates","value":true},
             {"condition":"minecraft:location_check","offsetX":1,
              "predicate":{"block":{"nbt":"{id:'minecraft:chest',x:1,y:64,z:0,Items:[{Slot:0b,id:'minecraft:diamond',count:3}]}"}}}
         ]}}
    ]);
    let mut config = datapack_config_with_enchantment_environment(
        "minecraft:looting",
        serde_json::json!({"minecraft:equipment_drops":entries}),
        &[("minecraft:plains", false), ("minecraft:desert", true)],
    );
    config.admin_lists = operator_config(&[("alice", 1)]).admin_lists;
    for (motion, biome, rider, expected) in [
        (0.0f64, "minecraft:desert", 1, false),
        (0.25, "minecraft:plains", 1, false),
        (0.25, "minecraft:desert", 2, false),
        (0.25, "minecraft:desert", 1, true),
    ] {
        let (sender, receiver) = mailbox();
        let mut world = spawn_pinned_world(config.clone());
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        assert!(world.set_noise_biome_for_test(
            Java26BuiltinDimension::Overworld,
            (0, 16, 0),
            biome
        ));
        let equip = console_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!(
                "item replace entity alice weapon.mainhand with diamond_sword[enchantments={{looting:3}},custom_data={{rider:{rider}}}]"
            ),
        );
        assert_eq!(
            super::super::entities::console_command_answer(&equip).0,
            Some(1)
        );
        let setup = console_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "setblock 1 64 0 chest{Items:[{Slot:0b,id:\"minecraft:diamond\",count:3}]}",
        );
        assert_eq!(
            super::super::entities::console_command_answer(&setup).0,
            Some(1)
        );
        let mut record = drop_chance_record(2.0);
        record.extend([
            NbtSummaryEntry {
                name: "NoAI".into(),
                value: NbtSummaryValue::Byte(1),
            },
            NbtSummaryEntry {
                name: "NoGravity".into(),
                value: NbtSummaryValue::Byte(1),
            },
            NbtSummaryEntry {
                name: "Motion".into(),
                value: NbtSummaryValue::List(vec![
                    NbtSummaryValue::Double(0.0f64.to_bits()),
                    NbtSummaryValue::Double(motion.to_bits()),
                    NbtSummaryValue::Double(0.0f64.to_bits()),
                ]),
            },
        ]);
        let victim = setup_victim_with_record(&mut world, &record);
        assert!(world.attach_external_player_for_test(ConnectionId(1), victim));
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "damage @e[type=zombie,limit=1] 40 minecraft:player_attack by alice",
        );
        assert_eq!(
            world
                .stored_items_for_test(Java26BuiltinDimension::Overworld)
                .iter()
                .any(|item| item.2 == "minecraft:diamond_sword"),
            expected,
            "motion {motion}, biome {biome}, rider {rider}"
        );
    }
}
