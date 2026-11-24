use super::feeding::{interact_entity_packet, stack_of};
use super::horse_interaction::record;
use super::*;

fn spawn_creeper(world: &mut ServerWorld) -> i32 {
    let position = world.publication().players[0].position;
    let kind = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CREEPER_ID;
    let id = world
        .spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            kind,
            Vec3d {
                z: position.z + 1.0,
                ..position
            },
            crate::entity::EntityData::GeneralLiving(GeneralLivingData::new(kind).unwrap()),
        )
        .0;
    let mut entries = record(world, id);
    entries.retain(|entry| entry.name != "NoAI" && entry.name != "Silent");
    for name in ["NoAI", "Silent"] {
        entries.push(bbb_protocol::NbtSummaryEntry {
            name: name.into(),
            value: bbb_protocol::NbtSummaryValue::Byte(1),
        });
    }
    world.load_stored_entity_record_for_test(Java26BuiltinDimension::Overworld, id, &entries);
    id
}

fn packet_for(event: &WorldEvent, connection_id: ConnectionId) -> Option<&ClientboundPlayPacket> {
    match event {
        WorldEvent::Send {
            connection_id: target,
            packet,
        } if *target == connection_id => Some(packet),
        WorldEvent::Broadcast { recipients, packet } if recipients.contains(&connection_id) => {
            Some(packet)
        }
        _ => None,
    }
}

fn any_packet(event: &WorldEvent) -> Option<&ClientboundPlayPacket> {
    match event {
        WorldEvent::Send { packet, .. } | WorldEvent::Broadcast { packet, .. } => Some(packet),
        _ => None,
    }
}

fn ignition_sound_for(events: &[WorldEvent], connection_id: ConnectionId, expected: &str) -> bool {
    events.iter().any(|event| {
        matches!(packet_for(event, connection_id),
            Some(ClientboundPlayPacket::Sound(sound))
                if sound.source == SoundSource::Hostile && sound.volume == 1.0
                    && (0.8..=1.2).contains(&sound.pitch)
                    && matches!(sound.sound, SoundEventHolder::Reference { registry_id }
                        if vanilla_sound_event_resource_id(registry_id) == Some(expected))
        )
    })
}

#[test]
fn live_creeper_ignition_syncs_state_sound_tool_damage_and_no_ai_explosion() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "igniter", 0xc0ee);
    join_loaded(&sender, &receiver, &mut world, 2, "observer", 0xc0ef);
    let id = spawn_creeper(&mut world);
    world.set_inventory_slot_for_test(ConnectionId(1), 0, stack_of("minecraft:flint_and_steel", 1));
    let result = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(id),
    );
    assert_eq!(
        result.snapshot.players[0].inventory.slots[0]
            .component_patch
            .damage,
        Some(1)
    );
    assert!(record(&world, id)
        .iter()
        .any(|entry| entry.name == "ignited"
            && entry.value == bbb_protocol::NbtSummaryValue::Byte(1)));
    assert!(result.events.iter().any(|event| matches!(any_packet(event),
        Some(ClientboundPlayPacket::SetEntityData(data))
            if data.id == id && data.values.iter().any(|row|
                row.data_id == bbb_protocol::entity_synched_data::VANILLA_CREEPER_IGNITED_DATA_ID
                    && row.value == bbb_protocol::packets::EntityDataValueKind::Boolean(true))
    )));
    // Creeper.java:214: level.playSound(player, ...) broadcasts to observers but excludes the actor.
    assert!(!ignition_sound_for(
        &result.events,
        ConnectionId(1),
        "minecraft:item.flintandsteel.use"
    ));
    assert!(ignition_sound_for(
        &result.events,
        ConnectionId(2),
        "minecraft:item.flintandsteel.use"
    ));
    assert!(
        !result.events.iter().any(|event| matches!(any_packet(event),
            Some(ClientboundPlayPacket::EntityAnimation(animation))
                if animation.id == world.player_entity_id_for_test(ConnectionId(1))
        ))
    );
    assert!(world
        .game_events(Java26BuiltinDimension::Overworld)
        .0
        .iter()
        .any(
            |event| event.game_event == VanillaGameEvent::ENTITY_INTERACT
                && event.source_entity_id == Some(world.player_entity_id_for_test(ConnectionId(1)))
        ));
    let mut exploded = false;
    for _ in 0..30 {
        let tick = world.tick(receiver.drain());
        exploded |= tick
            .events
            .iter()
            .any(|event| matches!(any_packet(event), Some(ClientboundPlayPacket::Explode(_))));
    }
    assert!(exploded);
    assert!(world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
        .is_none());
}

#[test]
fn live_creeper_fire_charges_repeat_ignition_and_creative_preserves_the_stack() {
    for game_type in [GameType::Survival, GameType::Creative] {
        let (sender, receiver) = mailbox();
        let mut config = datapack_config();
        config.game_type = game_type;
        let mut world = spawn_pinned_world(config);
        join_loaded(&sender, &receiver, &mut world, 1, "igniter", 0xc0ef);
        join_loaded(&sender, &receiver, &mut world, 2, "observer", 0xc0f9);
        let id = spawn_creeper(&mut world);
        world.set_inventory_slot_for_test(ConnectionId(1), 0, stack_of("minecraft:fire_charge", 3));
        for remaining in [2, 1] {
            let result = play(
                &sender,
                &receiver,
                &mut world,
                1,
                interact_entity_packet(id),
            );
            assert_eq!(
                result.snapshot.players[0].inventory.slots[0].count,
                if game_type == GameType::Creative {
                    3
                } else {
                    remaining
                }
            );
            assert!(!ignition_sound_for(
                &result.events,
                ConnectionId(1),
                "minecraft:item.firecharge.use"
            ));
            assert!(ignition_sound_for(
                &result.events,
                ConnectionId(2),
                "minecraft:item.firecharge.use"
            ));
        }
    }
}

#[test]
fn live_creeper_ignition_uses_the_current_item_tag() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "igniter", 0xc0f0);
    join_loaded(&sender, &receiver, &mut world, 2, "observer", 0xc0fa);
    let id = spawn_creeper(&mut world);
    world.install_level_registry_tags(Arc::new(crate::registry_tags::RegistryTagSnapshot::new([
        (
            "minecraft:item".to_owned(),
            "minecraft:creeper_igniters".to_owned(),
            vec!["minecraft:stick".to_owned()],
        ),
    ])));
    world.set_inventory_slot_for_test(ConnectionId(1), 0, stack_of("minecraft:stick", 2));
    let result = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(id),
    );
    assert_eq!(result.snapshot.players[0].inventory.slots[0].count, 2);
    assert!(!ignition_sound_for(
        &result.events,
        ConnectionId(1),
        "minecraft:item.flintandsteel.use"
    ));
    assert!(ignition_sound_for(
        &result.events,
        ConnectionId(2),
        "minecraft:item.flintandsteel.use"
    ));
    assert!(record(&world, id)
        .iter()
        .any(|entry| entry.name == "ignited"
            && entry.value == bbb_protocol::NbtSummaryValue::Byte(1)));
}

#[test]
fn live_creeper_ignition_stops_after_death() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "igniter", 0xc0f1);
    join_loaded(&sender, &receiver, &mut world, 2, "observer", 0xc0fb);
    let id = spawn_creeper(&mut world);
    world.set_inventory_slot_for_test(ConnectionId(1), 0, stack_of("minecraft:fire_charge", 3));
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(id),
    );
    let mut entries = record(&world, id);
    for entry in &mut entries {
        if entry.name == "Health" {
            entry.value = bbb_protocol::NbtSummaryValue::Float(0.0_f32.to_bits());
        } else if entry.name == "Fuse" {
            entry.value = bbb_protocol::NbtSummaryValue::Short(2);
        }
    }
    world.load_stored_entity_record_for_test(Java26BuiltinDimension::Overworld, id, &entries);
    let result = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(id),
    );
    assert_eq!(result.snapshot.players[0].inventory.slots[0].count, 2);
    assert!(!ignition_sound_for(
        &result.events,
        ConnectionId(2),
        "minecraft:item.firecharge.use"
    ));
    assert!(!result.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::Explode(_),
            ..
        }
    )));
    for _ in 0..3 {
        let tick = world.tick(receiver.drain());
        assert!(!tick.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::Explode(_),
                ..
            }
        )));
    }
}
