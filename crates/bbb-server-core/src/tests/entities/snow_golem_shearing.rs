use super::*;

#[test]
fn live_snow_golem_shearing_syncs_pumpkin_loot_durability_and_reload() {
    let dimension = Java26BuiltinDimension::Overworld;
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    let mut world = spawn_pinned_world(config.clone());
    join_loaded(&sender, &receiver, &mut world, 1, "shearer", 0x50ee);
    let position = world.publication().players[0].position;
    let kind = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID;
    let (id, _) = world.spawn_stored_entity_for_test(
        dimension,
        kind,
        Vec3d {
            z: position.z + 1.0,
            ..position
        },
        crate::entity::EntityData::GeneralLiving(GeneralLivingData::new(kind).unwrap()),
    );
    let uuid = world.stored_entity_for_test(dimension, id).unwrap().uuid();
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_SHEARS_ID),
            count: 1,
            component_patch: Default::default(),
        },
    );
    let interact = PlayServerbound::Interact(InteractEntity {
        entity_id: id,
        hand: InteractionHand::MainHand,
        location: Vec3d::default(),
        using_secondary_action: false,
    });
    let result = play(&sender, &receiver, &mut world, 1, interact.clone());
    assert_eq!(
        result.snapshot.players[0].inventory.slots[0]
            .component_patch
            .damage,
        Some(1)
    );
    assert!(result.events.iter().any(|event| matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(data), .. }
        if data.id == id && data.values.iter().any(|row|
            row.data_id == bbb_protocol::entity_synched_data::VANILLA_SNOW_GOLEM_PUMPKIN_DATA_ID
            && row.value == bbb_protocol::packets::EntityDataValueKind::Byte(0))
    )));
    assert!(result.events.iter().any(|event| matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. }
        if sound.source == SoundSource::Players && sound.volume == 1.0 && sound.pitch == 1.0
            && matches!(sound.sound, SoundEventHolder::Reference { registry_id }
                if vanilla_sound_event_resource_id(registry_id) == Some("minecraft:entity.snow_golem.shear"))
    )));
    assert!(!result.events.iter().any(|event| matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::EntityAnimation(animation), .. }
        if animation.id == world.player_entity_id_for_test(ConnectionId(1))
    )));
    let pumpkin =
        bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:carved_pumpkin")
            .unwrap()
            .id;
    let drops = |world: &ServerWorld| {
        world.stored_entity_ids_for_test(dimension).into_iter().filter(|id| {
        matches!(world.stored_entity_for_test(dimension, *id).unwrap().data(),
            crate::entity::EntityData::Item(item) if item.item().item_id() == Some(pumpkin) && item.item().count() == 1)
    }).count()
    };
    assert_eq!(drops(&world), 1);
    assert!(world
        .game_events(dimension)
        .0
        .iter()
        .any(|event| event.game_event == VanillaGameEvent::SHEAR
            && event.source_entity_id == Some(world.player_entity_id_for_test(ConnectionId(1)))));
    let again = play(&sender, &receiver, &mut world, 1, interact);
    assert_eq!(
        again.snapshot.players[0].inventory.slots[0]
            .component_patch
            .damage,
        Some(1)
    );
    assert_eq!(drops(&world), 1);
    assert!(!again.events.iter().any(|event| matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. }
        if matches!(sound.sound, SoundEventHolder::Reference { registry_id }
            if vanilla_sound_event_resource_id(registry_id) == Some("minecraft:entity.snow_golem.shear"))
    )));
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    let mut candidate = restore_candidate(10, None, Vec::new());
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let restored = ServerWorld::restore(config, &candidate).unwrap();
    let id = restored
        .live_entity_id_for_uuid_for_test(dimension, uuid)
        .unwrap();
    let Some(bbb_protocol::NbtSummaryValue::Compound(record)) =
        restored.stored_entity_command_data_for_test(dimension, id)
    else {
        panic!("the restored snow golem has an entity record")
    };
    assert!(record
        .iter()
        .any(|field| field.name == "Pumpkin"
            && field.value == bbb_protocol::NbtSummaryValue::Byte(0)));
}
