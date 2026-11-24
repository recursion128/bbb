use super::*;
use bbb_protocol::attribute_types::{VANILLA_ATTRIBUTE_MAX_HEALTH_ID, VANILLA_ATTRIBUTE_SCALE_ID};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;

const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn attribute(id: &str, base: i16, modifiers: Vec<NbtSummaryValue>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        nbt("id", NbtSummaryValue::String(id.into())),
        nbt("base", NbtSummaryValue::Short(base)),
        nbt("modifiers", NbtSummaryValue::List(modifiers)),
    ])
}

fn modifier(amount: i16) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        nbt("id", NbtSummaryValue::String("bbb:health".into())),
        nbt("amount", NbtSummaryValue::Short(amount)),
        nbt("operation", NbtSummaryValue::String("add_value".into())),
    ])
}

fn effect(id: &str, duration: i16, amplifier: NbtSummaryValue) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        nbt("id", NbtSummaryValue::String(id.into())),
        nbt("duration", NbtSummaryValue::Short(duration)),
        nbt("amplifier", amplifier),
    ])
}

fn living(world: &ServerWorld, uuid: Uuid) -> &crate::entity::LivingEntityState {
    let id = world
        .live_entity_id_for_uuid_for_test(DIMENSION, uuid)
        .expect("the raw cow reaches the live entity store");
    world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .living()
        .unwrap()
}

#[test]
fn entity_living_lists_raw_player_root_vehicle_joins_and_reopens() {
    const PLAYER_UUID: u128 = 0x221;
    const CONNECTION: ConnectionId = ConnectionId(1);
    let player_uuid = Uuid::from_u128(PLAYER_UUID);
    let root = std::env::temp_dir().join(format!(
        "bbb-entity-living-lists-root-vehicle-{}",
        std::process::id()
    ));
    let storage = created_fixture_save(&root, "bbb-living-root-vehicle");
    let paths = storage.paths().clone();
    let initial = storage.load_candidate().unwrap();
    let mut source = ServerWorld::restore(datapack_config(), &initial).unwrap();
    source.tick_rate_mut().set_frozen(true);
    let (sender, receiver) = mailbox();
    join_loaded(
        &sender,
        &receiver,
        &mut source,
        CONNECTION.0,
        "rider",
        PLAYER_UUID,
    );
    let position = source.publication().players[0].position;
    let (cow, cow_uuid) = source.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_COW_ID,
        position,
        EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_COW_ID).unwrap(),
        ),
    );
    assert!(source.attach_external_player_for_test(CONNECTION, cow));
    let request = source
        .save_request(1_786_000_022_010, 320, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(
        request.player_patches[0]
            .root_vehicle
            .as_ref()
            .unwrap()
            .attach()
            .unwrap(),
        cow_uuid
    );
    storage.save_snapshot(&initial, &request).unwrap();
    storage.close().unwrap();

    let player_store = PlayerDataStore::new(paths.clone(), WorldSaveLimits::default()).unwrap();
    let mut raw = player_store.load(player_uuid).unwrap().unwrap().document;
    let NbtSummaryValue::Compound(vehicle) = &mut raw
        .entries_mut()
        .iter_mut()
        .find(|entry| entry.name == "RootVehicle")
        .unwrap()
        .value
    else {
        panic!("saved player root vehicle")
    };
    let NbtSummaryValue::Compound(entity) = &mut vehicle
        .iter_mut()
        .find(|entry| entry.name == "Entity")
        .unwrap()
        .value
    else {
        panic!("embedded cow record")
    };
    entity.retain(|entry| {
        !matches!(
            entry.name.to_jdk_string().as_str(),
            "attributes" | "active_effects" | "Health"
        )
    });
    entity.extend([
        nbt("Health", nbt_float(100.0)),
        nbt(
            "attributes",
            NbtSummaryValue::List(vec![
                attribute("bbb:unknown", 2, vec![]),
                attribute("max_health", 24, vec![modifier(4)]),
                attribute("scale", 2, vec![]),
            ]),
        ),
        nbt(
            "active_effects",
            NbtSummaryValue::List(vec![
                effect("bbb:unknown", 20, NbtSummaryValue::Byte(0)),
                effect("speed", 400, NbtSummaryValue::Byte(1)),
            ]),
        ),
    ]);
    player_store.save(player_uuid, &raw).unwrap();

    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let candidate = storage.load_candidate().unwrap();
    let mut world = ServerWorld::restore(datapack_config(), &candidate).unwrap();
    world.tick_rate_mut().set_frozen(true);
    let (sender, receiver) = mailbox();
    join_loaded(
        &sender,
        &receiver,
        &mut world,
        CONNECTION.0,
        "rider",
        PLAYER_UUID,
    );
    let mounted = world
        .external_player_vehicle_state_for_test(CONNECTION)
        .2
        .unwrap();
    assert_eq!(
        world
            .stored_entity_for_test(DIMENSION, mounted)
            .unwrap()
            .uuid(),
        cow_uuid
    );
    let assert_runtime = |world: &ServerWorld, duration| {
        let cow = living(world, cow_uuid);
        assert_eq!(cow.max_health(), 28.0);
        assert_eq!(cow.health(), 28.0);
        assert_eq!(
            cow.attributes().value(VANILLA_ATTRIBUTE_SCALE_ID),
            Some(2.0)
        );
        let packed = cow.attributes().pack();
        let health = packed
            .iter()
            .find(|row| row.attribute_id == VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
            .unwrap();
        assert_eq!(health.base_value, 24.0);
        assert_eq!(health.modifiers.len(), 1);
        assert_eq!(health.modifiers[0].amount, 4.0);
        assert_eq!(cow.active_effects().count(), 1);
        let speed = cow.active_effect("minecraft:speed").unwrap();
        assert_eq!(speed.duration(), duration);
        assert_eq!(speed.amplifier(), 1);
    };
    // The player-mounted cow keeps ticking during the two-tick join handshake.
    assert_runtime(&world, 398);
    let packed = living(&world, cow_uuid).attributes().pack();
    let request = world
        .save_request(1_786_000_022_011, 321, RegionCompression::Zlib)
        .unwrap();
    assert!(request.player_patches[0].root_vehicle.is_some());
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let candidate = storage.load_candidate().unwrap();
    let mut restarted = ServerWorld::restore(datapack_config(), &candidate).unwrap();
    restarted.tick_rate_mut().set_frozen(true);
    let (sender, receiver) = mailbox();
    join_loaded(
        &sender,
        &receiver,
        &mut restarted,
        CONNECTION.0,
        "rider",
        PLAYER_UUID,
    );
    assert!(restarted
        .external_player_vehicle_state_for_test(CONNECTION)
        .2
        .is_some());
    assert_runtime(&restarted, 396);
    assert_eq!(living(&restarted, cow_uuid).attributes().pack(), packed);
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
