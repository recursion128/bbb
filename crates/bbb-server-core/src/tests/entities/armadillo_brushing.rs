use super::feeding::{animal_record_int, interact_entity_packet};
use super::horse_interaction::{hold, record};
use super::*;
use bbb_protocol::entity_synched_data::{
    VANILLA_ARMADILLO_STATE_DATA_ID, VANILLA_ARMADILLO_STATE_IDLE_ID,
    VANILLA_ARMADILLO_STATE_ROLLING_ID, VANILLA_ARMADILLO_STATE_SCARED_ID,
    VANILLA_ARMADILLO_STATE_UNROLLING_ID,
};
use bbb_protocol::packets::{EntityDataEnumSerializer, EntityDataValueKind};

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

#[test]
fn live_armadillo_retains_poses_refuses_scared_food_and_brushes_adults() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    let [x, y, z] = ServerWorld::fixture_spawn_block();
    let brush = serde_json::json!({
        "type":"minecraft:entity_interact",
        "pools":[{"rolls":1,"conditions":[
            {"condition":"minecraft:weather_check","raining":false,"thundering":false},
            {"condition":"minecraft:match_tool","predicate":{"items":"minecraft:brush", "components":{"minecraft:custom_data":{"marked":17}}, "predicates":{"minecraft:damage":{"damage":{"max":16}}}}},
            {"condition":"minecraft:entity_properties","entity":"target_entity","predicate":{"type":"minecraft:armadillo","location":{"block":{
                "blocks":"minecraft:chest",
                "nbt":format!("{{id:\"minecraft:chest\",x:{x},y:{y},z:{},Items:[{{Slot:0b,id:\"minecraft:diamond\",count:3}}]}}", z + 1)
            }}}},
            {"condition":"minecraft:entity_properties","entity":"interacting_entity","predicate":{"type":"minecraft:player","team":"brushers","slots":{"inventory.0":{"items":"minecraft:diamond","components":{"minecraft:custom_data":{"marked":29}}}},"equipment":{"mainhand":{"items":"minecraft:brush","components":{"minecraft:custom_data":{"marked":17}}}}}}
        ],"entries":[{"type":"minecraft:item","name":"minecraft:diamond","functions":[
            {"function":"minecraft:set_count","count":3},
            {"function":"minecraft:set_components","components":{"minecraft:custom_data":{"kept":23}}}
        ]}]}, {
            "rolls":1,"conditions":[{"condition":"minecraft:weather_check","raining":true}],
            "entries":[{"type":"minecraft:item","name":"minecraft:emerald"}]
        }]
    });
    let sheep = serde_json::json!({
        "type":"minecraft:shearing",
        "pools":[
            {"rolls":1,"conditions":[{"condition":"minecraft:weather_check","raining":false,"thundering":false}],
                "entries":[{"type":"minecraft:item","name":"minecraft:gold_ingot"}]},
            {"rolls":1,"conditions":[{"condition":"minecraft:weather_check","raining":true}],
                "entries":[{"type":"minecraft:item","name":"minecraft:emerald"}]}
        ]
    });
    let root = std::env::temp_dir().join(format!(
        "bbb-shearing-brush-weather-{}",
        uuid::Uuid::new_v4()
    ));
    let pack = root.join("weather");
    std::fs::create_dir_all(pack.join("data/minecraft/loot_table/brush")).unwrap();
    std::fs::create_dir_all(pack.join("data/minecraft/loot_table/shearing")).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"Shearing and brush weather","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
    for (path, table) in [
        ("data/minecraft/loot_table/brush/armadillo.json", brush),
        ("data/minecraft/loot_table/shearing/sheep.json", sheep),
    ] {
        std::fs::write(pack.join(path), serde_json::to_vec(&table).unwrap()).unwrap();
    }
    config.datapacks = Some(Arc::new(
        bbb_server_datapack::startup::DatapackStartup::load(
            &bbb_server_datapack::startup::DatapackStartupOptions {
                datapacks_dir: Some(root.clone()),
                saved_data_packs: None,
                safe_mode: false,
                limits: bbb_server_datapack::registry_data_loader::DatapackLoadLimits::default(),
            },
        )
        .unwrap(),
    ));
    std::fs::remove_dir_all(root).unwrap();
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "brusher", 0xa44a);
    let clear = console_command(&sender, &receiver, &mut world, 1, "weather clear");
    assert!(console_command_answer(&clear).0.is_some());
    world.tick_rate_mut().set_frozen(true);
    for command in ["team add brushers", "team join brushers brusher"] {
        let result = console_command(&sender, &receiver, &mut world, 1, command);
        assert_eq!(console_command_answer(&result).0, Some(1), "{command}");
    }
    let position = Vec3d {
        z: world.publication().players[0].position.z + 1.0,
        ..world.publication().players[0].position
    };
    let setup = console_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!(
            "setblock {} {} {} minecraft:chest{{Items:[{{Slot:0b,id:\"minecraft:diamond\",count:3}}]}}",
            position.x.floor() as i32,
            position.y.floor() as i32,
            position.z.floor() as i32,
        ),
    );
    assert_eq!(console_command_answer(&setup).0, Some(1));
    let kind = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMADILLO_ID;
    let (id, _) = world.spawn_stored_entity_for_test(
        DIM,
        kind,
        position,
        crate::entity::EntityData::GeneralLiving(GeneralLivingData::new(kind).unwrap()),
    );
    let mut entries = record(&world, id);
    let NbtSummaryValue::Compound(extra) =
        bbb_protocol::parse_snbt_compound("{NoAI:1b,scute_time:77}").unwrap()
    else {
        unreachable!()
    };
    for entry in extra {
        entries.retain(|field| field.name != entry.name);
        entries.push(entry);
    }
    world.load_stored_entity_record_for_test(DIM, id, &entries);
    for (name, expected_id) in [
        ("rolling", VANILLA_ARMADILLO_STATE_ROLLING_ID),
        ("scared", VANILLA_ARMADILLO_STATE_SCARED_ID),
        ("unrolling", VANILLA_ARMADILLO_STATE_UNROLLING_ID),
        ("idle", VANILLA_ARMADILLO_STATE_IDLE_ID),
    ] {
        let result = console_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("data merge entity @e[type=minecraft:armadillo,limit=1] {{state:\"{name}\"}}"),
        );
        assert_eq!(console_command_answer(&result).0, Some(1));
        let saved = record(&world, id);
        assert_eq!(
            saved.iter().filter(|entry| entry.name == "state").count(),
            1
        );
        assert!(saved
            .iter()
            .any(|entry| entry.name == "state"
                && entry.value == NbtSummaryValue::String(name.into())));
        assert_eq!(animal_record_int(&world, id, "scute_time"), 77);
        assert!(
            result.events.iter().any(|event| matches!(event,
                WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(data), .. }
                if data.id == id && data.values.iter().any(|row|
                    row.data_id == VANILLA_ARMADILLO_STATE_DATA_ID
                        && row.value == EntityDataValueKind::EnumId {
                            serializer: EntityDataEnumSerializer::ArmadilloState,
                            id: expected_id,
                        })
            )),
            "{name} reaches the player"
        );
        world.load_stored_entity_record_for_test(DIM, id, &saved);
        hold(&mut world, "minecraft:spider_eye", 2);
        let fed = play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact_entity_packet(id),
        );
        let scared = name != "idle";
        assert_eq!(
            fed.snapshot.players[0].inventory.slots[0].count,
            if scared { 2 } else { 1 }
        );
        assert_eq!(animal_record_int(&world, id, "InLove") > 0, !scared);
    }
    let reset = console_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "data merge entity @e[type=minecraft:armadillo,limit=1] {state:\"scared\",InLove:0}",
    );
    assert_eq!(console_command_answer(&reset).0, Some(1));
    hold(&mut world, "minecraft:brush", 1);
    let mut tool = world
        .player_inventory_for_test(ConnectionId(1))
        .selected_stack()
        .summary()
        .clone();
    tool.component_patch.custom_data =
        Some(bbb_protocol::parse_snbt_compound("{marked:17b}").unwrap());
    tool.component_patch
        .added_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID);
    world.set_inventory_slot_for_test(ConnectionId(1), 0, tool.clone());
    let mut slot_item = tool.clone();
    slot_item.item_id = Some(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID);
    slot_item.component_patch.custom_data =
        Some(bbb_protocol::parse_snbt_compound("{marked:29b}").unwrap());
    for (attempt, (damage, marked, expected_drops)) in
        [(16, 29, 1), (32, 30, 1), (48, 29, 1), (16, 29, 2)]
            .into_iter()
            .enumerate()
    {
        if attempt == 3 {
            world.set_inventory_slot_for_test(ConnectionId(1), 0, tool.clone());
        }
        slot_item.component_patch.custom_data =
            Some(bbb_protocol::parse_snbt_compound(&format!("{{marked:{marked}b}}")).unwrap());
        world.set_inventory_slot_for_test(ConnectionId(1), 9, slot_item.clone());
        let tick = play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact_entity_packet(id),
        );
        assert_eq!(
            tick.snapshot.players[0].inventory.slots[0]
                .component_patch
                .damage,
            Some(damage)
        );
        assert_eq!(
            world
                .stored_entity_ids_for_test(DIM)
                .into_iter()
                .filter(|id| matches!(
                    world.stored_entity_for_test(DIM, *id).unwrap().data(),
                    crate::entity::EntityData::Item(_)
                ))
                .count(),
            expected_drops
        );
        assert!(tick.events.iter().any(|event| matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. }
            if sound.source == SoundSource::Neutral && sound.volume == 1.0 && sound.pitch == 1.0
                && matches!(sound.sound, SoundEventHolder::Reference { registry_id }
                    if vanilla_sound_event_resource_id(registry_id) == Some("minecraft:entity.armadillo.brush"))
        )));
    }
    for source in [id, world.player_entity_id_for_test(ConnectionId(1))] {
        assert!(world.game_events(DIM).0.iter().any(|event| event.game_event
            == VanillaGameEvent::ENTITY_INTERACT
            && event.source_entity_id == Some(source)));
    }
    let drops: Vec<_> = world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .filter_map(|id| {
            let entity = world.stored_entity_for_test(DIM, id)?;
            let crate::entity::EntityData::Item(data) = entity.data() else {
                return None;
            };
            Some(data.item().clone())
        })
        .collect();
    assert_eq!(drops.len(), 2);
    for drop in drops {
        assert_eq!(
            drop.item_id(),
            Some(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID)
        );
        assert_eq!(drop.count(), 3);
        assert_eq!(
            drop.summary().component_patch.custom_data,
            Some(bbb_protocol::parse_snbt_compound("{kept:23b}").unwrap())
        );
    }
    let mut entries = record(&world, id);
    assert!(entries
        .iter()
        .any(|entry| entry.name == "scute_time" && entry.value == NbtSummaryValue::Int(77)));
    entries.retain(|entry| entry.name != "Age");
    entries.push(bbb_protocol::NbtSummaryEntry {
        name: "Age".into(),
        value: NbtSummaryValue::Int(-100),
    });
    world.load_stored_entity_record_for_test(DIM, id, &entries);
    let tick = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(id),
    );
    assert_eq!(
        tick.snapshot.players[0].inventory.slots[0]
            .component_patch
            .damage,
        Some(16)
    );

    let player_position = world.publication().players[0].position;
    let summoned = console_command(
        &sender,
        &receiver,
        &mut world,
        2,
        &format!(
            "summon minecraft:sheep {} {} {} {{NoAI:1b,NoGravity:1b,Tags:['weather_sheep']}}",
            player_position.x,
            player_position.y,
            player_position.z + 2.0,
        ),
    );
    assert_eq!(console_command_answer(&summoned).0, Some(1));
    let sheep = world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .find(|id| {
            let body = world.stored_entity_for_test(DIM, *id).unwrap();
            body.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SHEEP_ID
                && body.entity_tags().iter().any(|tag| tag == "weather_sheep")
        })
        .unwrap();
    let sheep_uuid = world.stored_entity_for_test(DIM, sheep).unwrap().uuid();
    hold(&mut world, "minecraft:shears", 1);
    let sheared = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(sheep),
    );
    let body = world.stored_entity_for_test(DIM, sheep).unwrap();
    assert_eq!(body.uuid(), sheep_uuid);
    assert!(
        matches!(body.data(), crate::entity::EntityData::GeneralLiving(data)
        if data.sheep_is_sheared() == Some(true))
    );
    assert_eq!(
        sheared.snapshot.players[0].inventory.slots[0]
            .component_patch
            .damage,
        Some(1)
    );
    assert!(sheared.events.iter().any(|event| matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. }
        if sound.source == SoundSource::Players
            && matches!(sound.sound, SoundEventHolder::Reference { registry_id }
                if vanilla_sound_event_resource_id(registry_id) == Some("minecraft:entity.sheep.shear"))
    )));
    let drops: Vec<_> = world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .filter_map(|id| match world.stored_entity_for_test(DIM, id)?.data() {
            crate::entity::EntityData::Item(data) => Some(data.item().clone()),
            _ => None,
        })
        .collect();
    assert_eq!(drops.len(), 3);
    assert_eq!(
        drops
            .iter()
            .filter(|stack| stack.item_id()
                == Some(bbb_protocol::item_types::VANILLA_ITEM_GOLD_INGOT_ID))
            .map(|stack| stack.count())
            .sum::<i32>(),
        1
    );
    assert!(drops
        .iter()
        .all(|stack| stack.item_id() != Some(bbb_protocol::item_types::VANILLA_ITEM_EMERALD_ID)));
}
