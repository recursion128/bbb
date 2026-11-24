use super::feeding::interact_entity_packet;
use super::horse_interaction::{hold, record};
use super::*;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_server_types::item_stack::RuntimeItemStack;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

#[test]
fn empty_hand_returns_a_copper_golems_complete_held_stack_towards_the_player() {
    let (sender, receiver, mut world) = super::horse_interaction::horse_world();
    world.tick_rate_mut().set_frozen(true);
    let position = Vec3d {
        z: world.publication().players[0].position.z + 1.0,
        ..world.publication().players[0].position
    };
    let kind = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID;
    let (id, _) = world.spawn_stored_entity_for_test(
        DIM,
        kind,
        position,
        crate::entity::EntityData::GeneralLiving(GeneralLivingData::new(kind).unwrap()),
    );
    let mut entries = record(&world, id);
    let NbtSummaryValue::Compound(extra) = bbb_protocol::parse_snbt_compound(
        r#"{equipment:{mainhand:{id:"minecraft:poppy",count:7,components:{"minecraft:custom_data":{kept:17}}}}}"#,
    ).unwrap() else { unreachable!() };
    for entry in extra {
        entries.retain(|field| field.name != entry.name);
        entries.push(entry);
    }
    world.load_stored_entity_record_for_test(DIM, id, &entries);
    let held = |world: &ServerWorld| {
        world
            .stored_entity_for_test(DIM, id)
            .unwrap()
            .living()
            .unwrap()
            .equipment()
            .get(EquipmentSlot::MainHand)
            .cloned()
    };
    let original = held(&world).unwrap();
    assert_eq!(original.count(), 7);
    assert!(original.summary().component_patch.custom_data.is_some());
    hold(&mut world, "minecraft:stone", 1);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(id),
    );
    assert_eq!(held(&world), Some(original.clone()));
    world.set_inventory_slot_for_test(ConnectionId(1), 0, ItemStackSummary::empty());
    for _ in 0..2 {
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact_entity_packet(id),
        );
        assert!(held(&world).is_none());
        let drops: Vec<_> = world
            .stored_entity_ids_for_test(DIM)
            .into_iter()
            .filter_map(|id| {
                let entity = world.stored_entity_for_test(DIM, id)?;
                let crate::entity::EntityData::Item(data) = entity.data() else {
                    return None;
                };
                Some((data.item().clone(), entity.delta_movement()))
            })
            .collect();
        assert_eq!(drops.len(), 1);
        assert_eq!(drops[0].0, original);
        assert!(drops[0].1.z < 0.0);
    }
}

#[test]
fn live_copper_golem_shearing_preserves_the_native_stack_and_uses_the_current_tag() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "shearer", 0xc090);
    world.tick_rate_mut().set_frozen(true);
    let position = Vec3d {
        z: world.publication().players[0].position.z + 1.0,
        ..world.publication().players[0].position
    };
    let kind = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID;
    let (id, _) = world.spawn_stored_entity_for_test(
        DIM,
        kind,
        position,
        crate::entity::EntityData::GeneralLiving(GeneralLivingData::new(kind).unwrap()),
    );
    let mut entries = record(&world, id);
    let NbtSummaryValue::Compound(extra) = bbb_protocol::parse_snbt_compound(
        r#"{NoAI:1b,equipment:{saddle:{id:"minecraft:poppy",count:7,components:{"minecraft:custom_data":{kept:17}}}}}"#,
    ).unwrap() else { unreachable!() };
    for entry in extra {
        entries.retain(|field| field.name != entry.name);
        entries.push(entry);
    }
    world.load_stored_entity_record_for_test(DIM, id, &entries);
    let original = world
        .stored_entity_for_test(DIM, id)
        .unwrap()
        .living()
        .unwrap()
        .equipment()
        .get(EquipmentSlot::Saddle)
        .unwrap()
        .clone();
    assert_eq!(original.count(), 7);
    assert!(original.summary().component_patch.custom_data.is_some());
    hold(&mut world, "minecraft:shears", 1);
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
        Some(1)
    );
    assert!(world
        .stored_entity_for_test(DIM, id)
        .unwrap()
        .living()
        .unwrap()
        .equipment()
        .get(EquipmentSlot::Saddle)
        .is_none());
    let drops = |world: &ServerWorld| {
        world
            .stored_entity_ids_for_test(DIM)
            .into_iter()
            .filter_map(|id| {
                let entity = world.stored_entity_for_test(DIM, id)?;
                let crate::entity::EntityData::Item(data) = entity.data() else {
                    return None;
                };
                Some((entity.position(), data.item().clone()))
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        drops(&world),
        vec![(
            Vec3d {
                y: position.y + 1.5,
                ..position
            },
            original.clone()
        )]
    );
    assert!(tick.events.iter().any(|event| matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. }
        if sound.source == SoundSource::Players && matches!(sound.sound,
            SoundEventHolder::Reference { registry_id }
            if vanilla_sound_event_resource_id(registry_id) == Some("minecraft:entity.copper_golem.shear"))
    )));
    assert!(world
        .game_events(DIM)
        .0
        .iter()
        .any(|event| event.game_event == VanillaGameEvent::SHEAR
            && event.source_entity_id == Some(world.player_entity_id_for_test(ConnectionId(1)))));
    world.install_level_registry_tags(Arc::new(crate::registry_tags::RegistryTagSnapshot::new([
        (
            "minecraft:item".into(),
            "minecraft:shearable_from_copper_golem".into(),
            vec!["minecraft:stick".into()],
        ),
    ])));
    world
        .stored_entity_mut_for_test(DIM, id)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_item_slot(EquipmentSlot::Saddle, Some(original));
    let refused = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(id),
    );
    assert_eq!(
        refused.snapshot.players[0].inventory.slots[0]
            .component_patch
            .damage,
        Some(1)
    );
    assert_eq!(drops(&world).len(), 1);
    let stick = RuntimeItemStack::from_summary(super::feeding::stack_of("minecraft:stick", 3));
    world
        .stored_entity_mut_for_test(DIM, id)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_item_slot(EquipmentSlot::Saddle, Some(stick.clone()));
    let accepted = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(id),
    );
    assert_eq!(
        accepted.snapshot.players[0].inventory.slots[0]
            .component_patch
            .damage,
        Some(2)
    );
    assert_eq!(drops(&world).len(), 2);
    assert!(drops(&world).iter().any(|(_, stack)| stack == &stick));
}

#[test]
fn live_copper_golem_waxing_and_scraping_sync_and_save_weather() {
    use bbb_protocol::entity_synched_data::{
        VANILLA_COPPER_GOLEM_WEATHER_STATE_DATA_ID, VANILLA_COPPER_GOLEM_WEATHER_STATE_EXPOSED_ID,
        VANILLA_COPPER_GOLEM_WEATHER_STATE_UNAFFECTED_ID,
        VANILLA_COPPER_GOLEM_WEATHER_STATE_WEATHERED_ID,
    };
    use bbb_protocol::packets::{EntityDataEnumSerializer, EntityDataValueKind};

    let (sender, receiver, mut world) = super::horse_interaction::horse_world();
    world.tick_rate_mut().set_frozen(true);
    let position = Vec3d {
        z: world.publication().players[0].position.z + 1.0,
        ..world.publication().players[0].position
    };
    let kind = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID;
    let (id, _) = world.spawn_stored_entity_for_test(
        DIM,
        kind,
        position,
        crate::entity::EntityData::GeneralLiving(GeneralLivingData::new(kind).unwrap()),
    );
    let mut entries = record(&world, id);
    entries.retain(|entry| {
        !matches!(
            entry.name.to_jdk_string().as_str(),
            "next_weather_age" | "weather_state"
        )
    });
    entries.push(bbb_protocol::NbtSummaryEntry {
        name: "weather_state".into(),
        value: NbtSummaryValue::String("weathered".into()),
    });
    entries.push(bbb_protocol::NbtSummaryEntry {
        name: "Silent".into(),
        value: NbtSummaryValue::Byte(1),
    });
    world.load_stored_entity_record_for_test(DIM, id, &entries);
    let assert_record =
        |world: &ServerWorld, age, state: &str| {
            let saved = record(world, id);
            assert!(saved.iter().any(|entry| entry.name == "next_weather_age"
                && entry.value == NbtSummaryValue::Long(age)));
            assert!(saved.iter().any(|entry| entry.name == "weather_state"
                && entry.value == NbtSummaryValue::String(state.into())));
        };
    let at = BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    };
    let has_level_event = |tick: &TickResult, event_type| {
        tick.events.iter().any(|event| {
            matches!(event,
                WorldEvent::Send { packet: ClientboundPlayPacket::LevelEvent(event), .. }
                if event.event_type == event_type && event.pos == at && event.data == 0
            )
        })
    };
    let has_weather = |tick: &TickResult, expected| {
        tick.events.iter().any(|event| {
            matches!(event,
                WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(data), .. }
                if data.id == id && data.values.iter().any(|value|
                    value.data_id == VANILLA_COPPER_GOLEM_WEATHER_STATE_DATA_ID
                        && value.value == EntityDataValueKind::EnumId {
                            serializer: EntityDataEnumSerializer::WeatheringCopperState,
                            id: expected,
                        })
            )
        })
    };
    hold(&mut world, "minecraft:honeycomb", 3);
    let waxed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(id),
    );
    assert_record(&world, -2, "weathered");
    assert!(has_weather(
        &waxed,
        VANILLA_COPPER_GOLEM_WEATHER_STATE_WEATHERED_ID
    ));
    assert_eq!(waxed.snapshot.players[0].inventory.slots[0].count, 2);
    assert!(has_level_event(&waxed, 3003));
    let saved = record(&world, id);
    world.load_stored_entity_record_for_test(DIM, id, &saved);
    let repeated = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(id),
    );
    assert_record(&world, -2, "weathered");
    assert_eq!(repeated.snapshot.players[0].inventory.slots[0].count, 2);
    assert!(!has_level_event(&repeated, 3003));
    hold(&mut world, "minecraft:iron_axe", 1);
    let scraped = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(id),
    );
    assert_record(&world, -1, "weathered");
    assert_eq!(
        scraped.snapshot.players[0].inventory.slots[0]
            .component_patch
            .damage,
        Some(1)
    );
    assert!(has_level_event(&scraped, 3004));
    assert!(scraped.events.iter().any(|event| matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. }
        if sound.source == SoundSource::Neutral && sound.volume == 1.0 && sound.pitch == 1.0
            && matches!(sound.sound, SoundEventHolder::Reference { registry_id }
            if vanilla_sound_event_resource_id(registry_id) == Some("minecraft:item.axe.scrape"))
    )));
    let saved = record(&world, id);
    world.load_stored_entity_record_for_test(DIM, id, &saved);
    assert_record(&world, -1, "weathered");
    for (damage, state, encoded) in [
        (2, "exposed", VANILLA_COPPER_GOLEM_WEATHER_STATE_EXPOSED_ID),
        (
            3,
            "unaffected",
            VANILLA_COPPER_GOLEM_WEATHER_STATE_UNAFFECTED_ID,
        ),
    ] {
        let tick = play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact_entity_packet(id),
        );
        assert_record(&world, -1, state);
        assert_eq!(
            tick.snapshot.players[0].inventory.slots[0]
                .component_patch
                .damage,
            Some(damage)
        );
        assert!(has_level_event(&tick, 3005));
        assert!(has_weather(&tick, encoded));
    }
    let saved = record(&world, id);
    world.load_stored_entity_record_for_test(DIM, id, &saved);
    assert_record(&world, -1, "unaffected");
    let repeated = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(id),
    );
    assert_eq!(
        repeated.snapshot.players[0].inventory.slots[0]
            .component_patch
            .damage,
        Some(3)
    );
    assert!(!has_level_event(&repeated, 3005));
}

fn load_weather(world: &mut ServerWorld, id: i32, extra: &str) {
    let mut entries = record(world, id);
    let NbtSummaryValue::Compound(extra) = bbb_protocol::parse_snbt_compound(extra).unwrap() else {
        unreachable!()
    };
    for entry in extra {
        entries.retain(|field| field.name != entry.name);
        entries.push(entry);
    }
    world.load_stored_entity_record_for_test(DIM, id, &entries);
}

fn saved_weather(world: &ServerWorld, id: i32) -> (String, i64) {
    let entries = record(world, id);
    let state = entries
        .iter()
        .find(|field| field.name == "weather_state")
        .unwrap();
    let age = entries
        .iter()
        .find(|field| field.name == "next_weather_age")
        .unwrap();
    let (NbtSummaryValue::String(state), NbtSummaryValue::Long(age)) = (&state.value, &age.value)
    else {
        panic!("saved copper golem weather");
    };
    (state.try_to_string().unwrap(), *age)
}

fn weather_golem(world: &mut ServerWorld) -> i32 {
    let kind = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID;
    let position = Vec3d {
        z: world.publication().players[0].position.z + 3.0,
        ..world.publication().players[0].position
    };
    world
        .spawn_stored_entity_for_test(
            DIM,
            kind,
            position,
            crate::entity::EntityData::GeneralLiving(GeneralLivingData::new(kind).unwrap()),
        )
        .0
}

#[test]
fn copper_golem_natural_weathering_ticks_without_ai_and_restores_its_deadline() {
    use bbb_protocol::entity_synched_data::{
        VANILLA_COPPER_GOLEM_WEATHER_STATE_DATA_ID, VANILLA_COPPER_GOLEM_WEATHER_STATE_EXPOSED_ID,
    };
    let (_, receiver, mut world) = super::horse_interaction::horse_world();
    let id = weather_golem(&mut world);
    load_weather(
        &mut world,
        id,
        r#"{NoAI:1b,NoGravity:1b,weather_state:"unaffected",next_weather_age:-1L}"#,
    );
    let start = world.game_time();
    world.tick(receiver.drain());
    let (state, deadline) = saved_weather(&world, id);
    assert_eq!(state, "unaffected");
    assert!((504000..=552001).contains(&(deadline - start)));
    let saved = record(&world, id);
    world.load_stored_entity_record_for_test(DIM, id, &saved);
    world.tick(receiver.drain());
    assert_eq!(saved_weather(&world, id), (state, deadline));
    world.set_game_time_for_test(deadline);
    let tick = world.tick(receiver.drain());
    let (state, next_deadline) = saved_weather(&world, id);
    assert_eq!(state, "exposed");
    assert!((504000..=552000).contains(&(next_deadline - deadline)));
    let synced = world.tick(receiver.drain());
    assert!(tick.events.iter().chain(&synced.events).any(|event| matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(data), .. }
        if data.id == id && data.values.iter().any(|value|
            value.data_id == VANILLA_COPPER_GOLEM_WEATHER_STATE_DATA_ID
            && matches!(value.value, EntityDataValueKind::EnumId { id: VANILLA_COPPER_GOLEM_WEATHER_STATE_EXPOSED_ID, .. }))
    )));
    let saved = record(&world, id);
    world.load_stored_entity_record_for_test(DIM, id, &saved);
    world.set_game_time_for_test(next_deadline);
    world.tick(receiver.drain());
    let (state, last_deadline) = saved_weather(&world, id);
    assert_eq!(state, "weathered");
    assert!((504000..=552000).contains(&(last_deadline - next_deadline)));
    load_weather(&mut world, id, "{next_weather_age:-2L}");
    world.set_game_time_for_test(last_deadline);
    world.tick(receiver.drain());
    assert_eq!(saved_weather(&world, id), ("weathered".into(), -2));
    load_weather(&mut world, id, "{next_weather_age:0L}");
    world.tick(receiver.drain());
    assert_eq!(saved_weather(&world, id), ("oxidized".into(), 0));
}

#[test]
fn copper_golem_becomes_named_statue_and_drops_preserved_stacks_and_gamerule_lead() {
    for (entity_drops, silent) in [(true, false), (false, true)] {
        let (sender, receiver) = mailbox();
        let mut config = datapack_config();
        config.game_type = GameType::Survival;
        config.keep_alive_interval_ticks = 30_000;
        let mut world = spawn_pinned_world(config);
        join_loaded(&sender, &receiver, &mut world, 1, "observer", 1);
        world.tick_rate_mut().set_frozen(true);
        for text in [
            "gamerule mob_drops false",
            "gamerule spawn_mobs false",
            if entity_drops {
                "gamerule entity_drops true"
            } else {
                "gamerule entity_drops false"
            },
        ] {
            let tick = console_command(&sender, &receiver, &mut world, 40, text);
            assert!(console_command_answer(&tick).0.is_some());
        }
        let id = weather_golem(&mut world);
        load_weather(
            &mut world,
            id,
            r#"{NoAI:1b,NoGravity:1b,Rotation:[90f,0f],CustomName:{text:"Patina",color:"gold"},weather_state:"oxidized",next_weather_age:-2L,equipment:{mainhand:{id:"minecraft:diamond",count:7,components:{"minecraft:custom_data":{kept:17}}},saddle:{id:"minecraft:poppy",count:1}},drop_chances:{mainhand:2f,saddle:2f}}"#,
        );
        load_weather(
            &mut world,
            id,
            if silent { "{Silent:1b}" } else { "{Silent:0b}" },
        );
        let entity = world.stored_entity_for_test(DIM, id).unwrap();
        let pos = entity.block_position();
        let name = entity.custom_name().unwrap().component().clone();
        let preserved = entity
            .living()
            .unwrap()
            .equipment()
            .get(EquipmentSlot::MainHand)
            .unwrap()
            .clone();
        hold(&mut world, "minecraft:lead", 1);
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact_entity_packet(id),
        );
        assert_eq!(
            crate::entity::leash::leash_holder(world.stored_entity_for_test(DIM, id).unwrap()),
            Some(world.player_entity_id_for_test(ConnectionId(1)))
        );
        world.set_raw_block_state_for_test(DIM, pos, block_state_id("minecraft:stone"));
        load_weather(&mut world, id, "{next_weather_age:0L}");
        world.tick_rate_mut().set_frozen(false);
        world.tick(receiver.drain());
        assert!(world.stored_entity_for_test(DIM, id).is_some());
        assert!(world.block_entity_save_for_test(DIM, pos).is_none());
        world.set_raw_block_state_for_test(DIM, pos, block_state_id("minecraft:air"));
        let mut sound = false;
        for _ in 0..3000 {
            let tick = world.tick(receiver.drain());
            sound |= tick.events.iter().any(|event| matches!(event,
                WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. }
                if matches!(sound.sound, SoundEventHolder::Reference { registry_id }
                    if vanilla_sound_event_resource_id(registry_id) == Some("minecraft:entity.copper_golem_become_statue"))
            ));
            if world.stored_entity_for_test(DIM, id).is_none() {
                break;
            }
        }
        assert!(world.stored_entity_for_test(DIM, id).is_none());
        assert_eq!(sound, !silent);
        let statue = world
            .get_block_entity(DIM, pos)
            .expect("live statue block entity");
        let state = statue.block_state();
        assert_eq!(
            bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state),
            Some(bbb_protocol::block_types::VANILLA_BLOCK_OXIDIZED_COPPER_GOLEM_STATUE_ID)
        );
        assert_eq!(
            bbb_protocol::block_state_types::vanilla_block_state_property(state, "facing"),
            Some("west")
        );
        let pose = crate::block_behavior::copper_golem_statue::pose(state).unwrap();
        assert!(
            bbb_protocol::block_state_types::vanilla_block_property_values(
                "minecraft:oxidized_copper_golem_statue",
                "copper_golem_pose"
            )
            .unwrap()
            .contains(&pose)
        );
        let saved = world.block_entity_save_for_test(DIM, pos).unwrap();
        let NbtSummaryValue::Compound(components) = &saved
            .iter()
            .find(|entry| entry.name == "components")
            .unwrap()
            .value
        else {
            panic!("statue components");
        };
        assert_eq!(
            components,
            &vec![bbb_protocol::NbtSummaryEntry {
                name: "minecraft:custom_name".into(),
                value: name
            }]
        );
        let items = world
            .stored_entity_ids_for_test(DIM)
            .into_iter()
            .filter_map(|id| {
                let entity = world.stored_entity_for_test(DIM, id)?;
                let crate::entity::EntityData::Item(data) = entity.data() else {
                    return None;
                };
                Some(data.item().clone())
            })
            .collect::<Vec<_>>();
        assert!(items.contains(&preserved));
        assert!(items
            .iter()
            .any(|item| item.item_resource_id() == Some("minecraft:poppy")));
        assert_eq!(
            items
                .iter()
                .filter(|item| item.item_resource_id() == Some("minecraft:lead"))
                .count(),
            usize::from(entity_drops)
        );
    }
}
