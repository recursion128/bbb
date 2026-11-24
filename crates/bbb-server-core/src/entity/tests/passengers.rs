use super::*;
use crate::connection::ConnectionId;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};

use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID;
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_types::{
    vanilla_entity_type_passenger_attachments, vanilla_entity_type_vehicle_attachment,
    VANILLA_ENTITY_TYPE_BAMBOO_RAFT_ID, VANILLA_ENTITY_TYPE_BREEZE_ID,
    VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, VANILLA_ENTITY_TYPE_CAMEL_ID,
    VANILLA_ENTITY_TYPE_CHICKEN_ID, VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_GOAT_ID,
    VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, VANILLA_ENTITY_TYPE_HORSE_ID, VANILLA_ENTITY_TYPE_LLAMA_ID,
    VANILLA_ENTITY_TYPE_OAK_BOAT_ID, VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
    VANILLA_ENTITY_TYPE_PIG_ID, VANILLA_ENTITY_TYPE_SLIME_ID, VANILLA_ENTITY_TYPE_STRIDER_ID,
    VANILLA_ENTITY_TYPE_VILLAGER_ID, VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};

fn joined_players(count: usize) -> (NativeLevel, Vec<i32>) {
    let mut level = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    let players = (1..=count)
        .map(|index| {
            crate::tests::join_loaded(
                &sender,
                &receiver,
                &mut level.world,
                index as u64,
                &format!("rider{index}"),
                index as u128,
            );
            level
                .world
                .player_entity_id_for_test(ConnectionId(index as u64))
        })
        .collect();
    (level, players)
}

fn unoccupied_ids(level: &NativeLevel) -> EntityIdCounter {
    let ids = EntityIdCounter::new();
    let highest = level
        .world
        .publication()
        .players
        .iter()
        .map(|player| player.entity_id)
        .chain(level.world.stored_entity_ids_for_test(DIMENSION))
        .max()
        .unwrap_or(0);
    for _ in 0..highest {
        ids.next_id();
    }
    ids
}

fn position() -> Vec3d {
    Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    }
}

fn rotated(point: [f32; 3], yaw: f32) -> Vec3d {
    let radians = -f64::from(yaw).to_radians();
    let (sin, cos) = radians.sin_cos();
    let x = f64::from(point[0]);
    let z = f64::from(point[2]);
    Vec3d {
        x: x * cos + z * sin,
        y: f64::from(point[1]),
        z: z * cos - x * sin,
    }
}

fn assert_f64_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1.0e-6,
        "expected {expected}, got {actual}"
    );
}

fn assert_f32_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1.0e-5,
        "expected {expected}, got {actual}"
    );
}

fn link_passenger(store: &mut EntityStore, vehicle_id: i32, passenger_id: i32) -> bool {
    store.link_existing_passenger(vehicle_id, passenger_id)
}

#[test]
fn visibility_paused_passenger_keeps_relation_until_ticking_resumes() {
    let mut native = NativeLevel::new();
    let mut store = EntityStore::new_managed(
        crate::random::GameRandom::with_seed(15),
        EntityIdCounter::new(),
    );
    store.apply_chunk_visibility((0, 0), crate::entity::EntityVisibility::Ticking);
    store.apply_chunk_visibility((1, 0), crate::entity::EntityVisibility::Tracked);
    let root = crate::entity::spawn_armor_stand(&mut store, position()).unwrap();
    let passenger = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_PIG_ID, position())
        .unwrap();
    assert!(link_passenger(&mut store, root, passenger));
    let paused = Vec3d {
        x: 16.5,
        ..position()
    };
    store.get_mut(passenger).unwrap().set_pos(paused);
    store.refile(passenger);
    native.with_level(|ctx| tick_entities(&mut store, ctx, true));
    assert_eq!(store.get(passenger).unwrap().position(), paused);
    assert_eq!(store.vehicle(passenger), Some(root));
    store.apply_chunk_visibility((1, 0), crate::entity::EntityVisibility::Ticking);
    native.with_level(|ctx| tick_entities(&mut store, ctx, true));
    assert_ne!(store.get(passenger).unwrap().position(), paused);
    assert_eq!(store.vehicle(passenger), Some(root));
}

#[test]
fn external_player_vehicle_normal_capacity_and_forced_restore_are_distinct() {
    let (native, players) = joined_players(8);
    let mut store = EntityStore::new(0x4558_5443, unoccupied_ids(&native));
    let minecart = crate::entity::spawn_minecart(&mut store, position(), 0.0).unwrap();
    assert!(!store.attach_test_player(
        players[0],
        minecart,
        crate::entity::ExternalPlayerCanRide::Denied,
    ));
    assert!(store.attach_test_player(
        players[0],
        minecart,
        crate::entity::ExternalPlayerCanRide::Allowed
    ));
    assert!(!store.attach_test_player(
        players[1],
        minecart,
        crate::entity::ExternalPlayerCanRide::Allowed
    ));
    assert_eq!(store.detach_external_player(players[0]), Some(minecart));

    let chicken = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_CHICKEN_ID, position())
        .unwrap();
    assert!(store.attach_test_player(
        players[2],
        chicken,
        crate::entity::ExternalPlayerCanRide::Allowed
    ));
    assert!(!store.attach_test_player(
        players[3],
        chicken,
        crate::entity::ExternalPlayerCanRide::Allowed
    ));

    let boat =
        crate::entity::spawn_boat(&mut store, VANILLA_ENTITY_TYPE_OAK_BOAT_ID, position(), 0.0)
            .unwrap();
    assert!(store.attach_test_player(
        players[4],
        boat,
        crate::entity::ExternalPlayerCanRide::Allowed
    ));
    assert!(store.attach_test_player(
        players[5],
        boat,
        crate::entity::ExternalPlayerCanRide::Allowed
    ));
    assert!(!store.attach_test_player(
        players[6],
        boat,
        crate::entity::ExternalPlayerCanRide::Allowed
    ));

    let chest = crate::entity::spawn_chest_boat(
        &mut store,
        VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
        position(),
        0.0,
    )
    .unwrap();
    let passenger = crate::entity::spawn_armor_stand(&mut store, position()).unwrap();
    assert!(link_passenger(&mut store, chest, passenger));
    assert!(!store.attach_test_player(
        players[7],
        chest,
        crate::entity::ExternalPlayerCanRide::Allowed
    ));
    assert_eq!(store.passengers(chest), &[passenger]);
}

#[test]
fn external_player_vehicle_controller_and_seat_placement_use_live_riders() {
    let (native, players) = joined_players(3);
    let mut store = EntityStore::new(0x4558_5458, unoccupied_ids(&native));
    let boat =
        crate::entity::spawn_boat(&mut store, VANILLA_ENTITY_TYPE_OAK_BOAT_ID, position(), 0.0)
            .unwrap();
    let stored = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_PIG_ID, position())
        .unwrap();
    assert!(link_passenger(&mut store, boat, stored));
    store.drain_lifecycle_events();

    let player = players[0];
    assert!(store.attach_test_player(player, boat, crate::entity::ExternalPlayerCanRide::Allowed));
    assert_eq!(store.passengers(boat), &[player, stored]);
    assert_eq!(
        store.external_player_direct_controlled_boat(player),
        Some(boat)
    );
    assert!(matches!(
        store.drain_lifecycle_events().as_slice(),
        [EntityLifecycleEvent::PassengersChanged {
            vehicle_id,
            passenger_ids,
        }] if *vehicle_id == boat && passenger_ids == &[player, stored]
    ));
    let placement = store
        .external_player_vehicle_placement(player, 0.0)
        .unwrap();
    let player_vehicle_offset = vanilla_entity_type_vehicle_attachment(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
    )
    .unwrap();
    assert_f64_close(
        placement.position.z - position().z + f64::from(player_vehicle_offset[2]),
        0.2,
    );

    let carrier = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, position())
        .unwrap();
    let first_player = players[1];
    let second_player = players[2];
    assert!(store.attach_test_player(
        first_player,
        carrier,
        crate::entity::ExternalPlayerCanRide::Allowed,
    ));
    let middle = crate::entity::spawn_armor_stand(&mut store, position()).unwrap();
    assert!(link_passenger(&mut store, carrier, middle));
    assert!(store.attach_test_player(
        second_player,
        carrier,
        crate::entity::ExternalPlayerCanRide::Allowed,
    ));
    assert_eq!(
        store.passengers(carrier),
        &[first_player, middle, second_player]
    );
}

#[test]
fn camel_passenger_geometry_uses_index_age_and_scale_and_horse_uses_zero_stand_state() {
    let mut store = EntityStore::new(0x4558_5459, EntityIdCounter::new());
    store.set_game_time(100);
    let camel = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_CAMEL_ID, position())
        .unwrap();
    {
        let camel = store.get_mut(camel).unwrap();
        let EntityData::GeneralLiving(data) = &mut camel.data else {
            panic!("camel must use GeneralLiving");
        };
        assert!(data.ageable_retained_mut().unwrap().set_age(-100));
    }
    {
        let camel = store.get_mut(camel).unwrap();
        camel
            .living_mut()
            .unwrap()
            .attributes_mut()
            .get_instance(VANILLA_ATTRIBUTE_SCALE_ID)
            .unwrap()
            .set_base_value(2.0);
    }
    let driver = crate::entity::spawn_armor_stand(&mut store, position()).unwrap();
    let animal = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_PIG_ID, position())
        .unwrap();
    let rear = crate::entity::spawn_armor_stand(&mut store, position()).unwrap();
    assert!(link_passenger(&mut store, camel, driver));
    assert!(link_passenger(&mut store, camel, animal));
    assert!(link_passenger(&mut store, camel, rear));

    let mut level = NativeLevel::new();
    level.world.set_game_time_for_test(100);
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    let camel_entity = store.get(camel).unwrap();
    let camel_position = camel_entity.position();
    let camel_yaw = camel_entity.y_rot();
    for (passenger_id, expected_z) in [(driver, 0.6), (animal, -0.6), (rear, -0.84)] {
        let passenger = store.get(passenger_id).unwrap();
        let rider_offset = vanilla_entity_type_vehicle_attachment(passenger.entity_type_id())
            .map(|point| rotated(point, passenger.y_rot()))
            .unwrap();
        let expected = rotated([0.0, 2.4, expected_z], camel_yaw);
        assert_f64_close(
            passenger.position().x - camel_position.x + rider_offset.x,
            expected.x,
        );
        assert_f64_close(
            passenger.position().y - camel_position.y + rider_offset.y,
            expected.y,
        );
        assert_f64_close(
            passenger.position().z - camel_position.z + rider_offset.z,
            expected.z,
        );
    }

    let camel_husk = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, position())
        .unwrap();
    let EntityData::GeneralLiving(data) = &mut store.get_mut(camel_husk).unwrap().data else {
        panic!("camel husk must use GeneralLiving");
    };
    assert!(data.ageable_retained_mut().unwrap().set_age(-100));
    let (height, scale) = crate::entity::camel::passenger_attachment_height_and_scale(
        store.get(camel_husk).unwrap(),
        true,
        100,
    )
    .unwrap();
    assert_f32_close(height, 2.0);
    assert_f32_close(scale, 1.0);

    let horse = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_HORSE_ID, position())
        .unwrap();
    let passenger = crate::entity::spawn_armor_stand(&mut store, position()).unwrap();
    assert!(crate::entity::passenger::relation_supported(
        crate::entity::passenger::RelationAdmission::Normal,
        store.get(horse).unwrap(),
        store.get(passenger).unwrap(),
        0,
        1,
        store.game_time(),
    ));
    assert!(crate::entity::passenger::relation_supported(
        crate::entity::passenger::RelationAdmission::Forced,
        store.get(horse).unwrap(),
        store.get(passenger).unwrap(),
        0,
        1,
        store.game_time(),
    ));
}

/// `Entity.startRiding` (`Entity.java:2353-2391`) admits a rider whenever
/// `canRide` (`Entity.java:2393-2395`) and `canAddPassenger`
/// (`Entity.java:2452-2454`) agree; neither the pig nor the chicken overrides
/// either, so an empty pig takes a chicken and refuses the second rider.
#[test]
fn generated_living_vehicle_scales_its_rider_attachment() {
    for (entity_type, baby, scale, age_scale) in [
        (VANILLA_ENTITY_TYPE_PIG_ID, false, 1.0, 1.0),
        (VANILLA_ENTITY_TYPE_PIG_ID, true, 1.0, 0.5),
        (VANILLA_ENTITY_TYPE_PIG_ID, false, 2.0, 1.0),
        (VANILLA_ENTITY_TYPE_GOAT_ID, true, 1.0, 0.55),
    ] {
        let mut store = EntityStore::new(0x4558_545d, EntityIdCounter::new());
        let pig = store.spawn_general_living(entity_type, position()).unwrap();
        let entity = store.get_mut(pig).unwrap();
        if let EntityData::GeneralLiving(data) = &mut entity.data {
            data.ageable_retained_mut()
                .unwrap()
                .set_age(if baby { -100 } else { 0 });
        }
        entity
            .living_mut()
            .unwrap()
            .attributes_mut()
            .get_instance(VANILLA_ATTRIBUTE_SCALE_ID)
            .unwrap()
            .set_base_value(scale);
        let chicken = store
            .spawn_general_living(VANILLA_ENTITY_TYPE_CHICKEN_ID, position())
            .unwrap();
        assert!(link_passenger(&mut store, pig, chicken));
        assert_eq!(store.passengers(pig), [chicken]);
        assert_eq!(store.vehicle(chicken), Some(pig));

        let mut level = NativeLevel::new();
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
        let attachment = vanilla_entity_type_passenger_attachments(entity_type).unwrap();
        let rider_offset =
            vanilla_entity_type_vehicle_attachment(VANILLA_ENTITY_TYPE_CHICKEN_ID).unwrap();
        let pig_entity = store.get(pig).unwrap();
        let pig_position = pig_entity.position();
        let expected = rotated(
            attachment[0].map(|coordinate| coordinate * scale as f32 * age_scale),
            pig_entity.y_rot(),
        );
        let seated = store.get(chicken).unwrap();
        let rider = rotated(rider_offset, seated.y_rot());
        let seated = seated.position();
        assert_f64_close(seated.x, pig_position.x + expected.x - rider.x);
        assert_f64_close(seated.y, pig_position.y + expected.y - rider.y);
        assert_f64_close(seated.z, pig_position.z + expected.z - rider.z);
        // `EntityType.java:789-791` declares the pig's single passenger attachment.
        if entity_type == VANILLA_ENTITY_TYPE_PIG_ID {
            assert_f32_close(attachment[0][1], 0.868_75);
        }

        let second = store
            .spawn_general_living(VANILLA_ENTITY_TYPE_CHICKEN_ID, position())
            .unwrap();
        assert!(!link_passenger(&mut store, pig, second));
    }
}

#[test]
fn typed_living_attachment_overrides_use_owned_runtime_state() {
    let mut store = EntityStore::new(0x4558_545b, EntityIdCounter::new());
    let llama = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_LLAMA_ID, position())
        .unwrap();
    let strider = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_STRIDER_ID, position())
        .unwrap();
    for vehicle_id in [llama, strider] {
        let EntityData::GeneralLiving(data) = &mut store.get_mut(vehicle_id).unwrap().data else {
            panic!("typed vehicle must use GeneralLiving");
        };
        assert!(data.ageable_retained_mut().unwrap().set_age(-100));
        store
            .get_mut(vehicle_id)
            .unwrap()
            .living_mut()
            .unwrap()
            .attributes_mut()
            .get_instance(VANILLA_ATTRIBUTE_SCALE_ID)
            .unwrap()
            .set_base_value(1.5);
    }
    let slime = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_SLIME_ID, position())
        .unwrap();
    store
        .get_mut(slime)
        .unwrap()
        .living_mut()
        .unwrap()
        .attributes_mut()
        .get_instance(VANILLA_ATTRIBUTE_SCALE_ID)
        .unwrap()
        .set_base_value(2.0);

    let llama_rider = crate::entity::spawn_armor_stand(&mut store, position()).unwrap();
    let strider_rider = crate::entity::spawn_armor_stand(&mut store, position()).unwrap();
    let slime_rider = crate::entity::spawn_armor_stand(&mut store, position()).unwrap();
    assert!(link_passenger(&mut store, llama, llama_rider));
    assert!(store.link_existing_passenger(strider, strider_rider,));
    assert!(link_passenger(&mut store, slime, slime_rider));

    NativeLevel::new().with_level(|ctx| tick_entities(&mut store, ctx, true));
    let expected = [
        (llama, llama_rider, [0.0, 1.215, -0.225]),
        (strider, strider_rider, [0.0, 1.275, 0.0]),
        (slime, slime_rider, [0.0, (0.52 - 0.015_625) * 2.0, 0.0]),
    ];
    for (vehicle_id, passenger_id, local_offset) in expected {
        let vehicle = store.get(vehicle_id).unwrap();
        let passenger = store.get(passenger_id).unwrap();
        let rider_offset = vanilla_entity_type_vehicle_attachment(passenger.entity_type_id())
            .map(|point| rotated(point, passenger.y_rot()))
            .unwrap();
        let expected_offset = rotated(local_offset, vehicle.y_rot());
        assert_f64_close(
            passenger.position().x - vehicle.position().x + rider_offset.x,
            expected_offset.x,
        );
        assert_f64_close(
            passenger.position().y - vehicle.position().y + rider_offset.y,
            expected_offset.y,
        );
        assert_f64_close(
            passenger.position().z - vehicle.position().z + rider_offset.z,
            expected_offset.z,
        );
    }
}

#[test]
fn external_player_vehicle_normal_admission_uses_exact_subtype_state() {
    let (native, players) = joined_players(14);
    let mut store = EntityStore::new(0x4558_5444, unoccupied_ids(&native));
    let happy = store
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
            position(),
        )
        .unwrap();
    for player_id in players[0..4].iter().copied() {
        assert!(store.attach_test_player(
            player_id,
            happy,
            crate::entity::ExternalPlayerCanRide::Allowed,
        ));
    }
    assert!(!store.attach_test_player(
        players[4],
        happy,
        crate::entity::ExternalPlayerCanRide::Allowed
    ));

    let camel = store
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAMEL_ID,
            position(),
        )
        .unwrap();
    for player_id in players[5..8].iter().copied() {
        assert!(store.attach_test_player(
            player_id,
            camel,
            crate::entity::ExternalPlayerCanRide::Allowed,
        ));
    }
    assert!(!store.attach_test_player(
        players[8],
        camel,
        crate::entity::ExternalPlayerCanRide::Allowed
    ));

    let nautilus = store
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_NAUTILUS_ID,
            position(),
        )
        .unwrap();
    assert!(store.attach_test_player(
        players[9],
        nautilus,
        crate::entity::ExternalPlayerCanRide::Allowed
    ));
    assert!(!store.attach_test_player(
        players[10],
        nautilus,
        crate::entity::ExternalPlayerCanRide::Allowed
    ));

    let strider = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_STRIDER_ID, position())
        .unwrap();
    assert!(store.attach_test_player(
        players[11],
        strider,
        crate::entity::ExternalPlayerCanRide::Allowed
    ));

    let marker =
        crate::entity::spawn_marker(&mut store, position(), crate::entity::MarkerData::new())
            .unwrap();
    assert!(!store.attach_test_player(
        players[12],
        marker,
        crate::entity::ExternalPlayerCanRide::Allowed
    ));
    let ominous = crate::entity::spawn_ominous_item_spawner(
        &mut store,
        position(),
        crate::entity::item::plain_stack("minecraft:stone", 1).unwrap(),
    )
    .unwrap();
    assert!(!store.attach_test_player(
        players[13],
        ominous,
        crate::entity::ExternalPlayerCanRide::Allowed
    ));
}

#[test]
fn an_armor_stand_vehicle_seats_its_rider_at_its_own_height() {
    let mut store = EntityStore::new(0x4558_5445, EntityIdCounter::new());
    let vehicle = crate::entity::spawn_armor_stand(&mut store, position()).unwrap();
    assert!(vanilla_entity_type_passenger_attachments(
        store.get(vehicle).unwrap().entity_type_id()
    )
    .is_none());
    let passenger = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_PIG_ID, position())
        .unwrap();
    assert!(store.link_existing_passenger(vehicle, passenger));
    NativeLevel::new().with_level(|ctx| tick_entities(&mut store, ctx, true));
    assert_f64_close(
        store.get(passenger).unwrap().position().y,
        store.get(vehicle).unwrap().position().y
            + f64::from(store.get(vehicle).unwrap().bb_height()),
    );
}

#[test]
fn a_spider_sinks_into_a_narrower_vehicle() {
    for (spider_type, sink) in [
        (
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SPIDER_ID,
            0.3125,
        ),
        (
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID,
            0.21875,
        ),
    ] {
        let mut store = EntityStore::new(0x4558_5446, EntityIdCounter::new());
        let vehicle = crate::entity::spawn_armor_stand(&mut store, position()).unwrap();
        let passenger = store.spawn_general_living(spider_type, position()).unwrap();
        assert!(store.link_existing_passenger(vehicle, passenger));
        NativeLevel::new().with_level(|ctx| tick_entities(&mut store, ctx, true));
        let vehicle = store.get(vehicle).unwrap();
        assert_f64_close(
            store.get(passenger).unwrap().position().y,
            vehicle.position().y + f64::from(vehicle.bb_height()) - sink,
        );
    }
}

#[test]
fn external_player_vehicle_root_control_placement_and_pose_update_are_typed() {
    let (native, players) = joined_players(1);
    let mut store = EntityStore::new(0x4558_5450, unoccupied_ids(&native));
    let root = crate::entity::spawn_minecart(&mut store, position(), 0.0).unwrap();
    let boat_position = Vec3d {
        x: 4.5,
        ..position()
    };
    let boat = crate::entity::spawn_boat(
        &mut store,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        boat_position,
        30.0,
    )
    .unwrap();
    assert!(crate::entity::boat::set_delta_rotation_for_test(
        store.get_mut(boat).unwrap(),
        20.0,
    ));
    assert!(store.link_existing_passenger(root, boat));
    let player_id = players[0];
    assert!(store.attach_test_player(
        player_id,
        boat,
        crate::entity::ExternalPlayerCanRide::Allowed
    ));
    assert_eq!(store.external_player_vehicle(player_id), Some(boat));
    assert_eq!(store.external_player_root_vehicle(player_id), Some(root));
    assert_eq!(
        store.external_player_direct_controlled_boat(player_id),
        Some(boat)
    );
    let (attach_uuid, root_record) =
        crate::entity::write_external_player_root_vehicle(&store, player_id)
            .unwrap()
            .unwrap();
    assert_eq!(attach_uuid, store.get(boat).unwrap().uuid());
    assert_eq!(root_record.as_ref().uuid(), store.get(root).unwrap().uuid());
    assert!(store.set_external_player_boat_paddles(player_id, true, false));
    assert_eq!(
        crate::entity::boat::boat(store.get(boat).unwrap().data())
            .unwrap()
            .paddle_state(),
        (true, false)
    );

    let placement = store
        .external_player_vehicle_placement(player_id, 170.0)
        .unwrap();
    let rider_offset = rotated(
        vanilla_entity_type_vehicle_attachment(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
        )
        .unwrap(),
        170.0,
    );
    assert_f64_close(placement.position.x, boat_position.x - rider_offset.x);
    assert_f64_close(
        placement.position.y,
        boat_position.y + f64::from(store.get(boat).unwrap().bb_height() / 3.0) - rider_offset.y,
    );
    assert_f64_close(placement.position.z, boat_position.z - rider_offset.z);
    assert_f32_close(placement.y_rot, 135.0);

    let moved = Vec3d {
        x: 40.5,
        y: 70.0,
        z: -8.5,
    };
    assert!(!store.update_external_player_root_controlled_boat_pose(player_id, moved, 42.0, 3.0,));
    assert_eq!(store.get(boat).unwrap().position(), boat_position);
    store.drain_lifecycle_events();
    assert_eq!(store.detach_external_player(player_id), Some(boat));
    assert_eq!(
        store.drain_lifecycle_events(),
        vec![
            EntityLifecycleEvent::PassengersChanged {
                vehicle_id: boat,
                passenger_ids: Vec::new(),
            },
            EntityLifecycleEvent::ExternalPlayerBoardingCooldownReset {
                player_entity_id: player_id,
                dismount: None,
            },
        ]
    );
    assert!(!store.set_external_player_boat_paddles(player_id, false, false));
    assert_eq!(store.external_player_vehicle(player_id), None);
    assert!(store.passengers(boat).is_empty());

    let standalone = crate::entity::spawn_boat(
        &mut store,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        Vec3d {
            x: 12.5,
            ..position()
        },
        0.0,
    )
    .unwrap();
    assert!(store.attach_test_player(
        player_id,
        standalone,
        crate::entity::ExternalPlayerCanRide::Allowed,
    ));
    assert!(store.update_external_player_root_controlled_boat_pose(player_id, moved, 42.0, 3.0,));
    assert_eq!(store.get(standalone).unwrap().position(), moved);
    assert_f32_close(store.get(standalone).unwrap().y_rot(), 42.0);
}

#[test]
fn external_player_vehicle_disconnect_unloads_only_a_sole_player_root_tree() {
    let (native, players) = joined_players(3);
    let mut store = EntityStore::new(0x4558_5455, unoccupied_ids(&native));
    let boat =
        crate::entity::spawn_boat(&mut store, VANILLA_ENTITY_TYPE_OAK_BOAT_ID, position(), 0.0)
            .unwrap();
    let first_player = players[0];
    let second_player = players[1];
    assert!(store.attach_test_player(
        first_player,
        boat,
        crate::entity::ExternalPlayerCanRide::Allowed,
    ));
    assert!(store.attach_test_player(
        second_player,
        boat,
        crate::entity::ExternalPlayerCanRide::Allowed,
    ));
    store.drain_lifecycle_events();
    assert_eq!(
        write_entity_chunk(&store, (0, 0)).unwrap().entities().len(),
        1
    );

    assert_eq!(
        store.unload_external_player_vehicle(first_player),
        Some(crate::entity::ExternalPlayerVehicleUnload::DetachedOnly {
            direct_vehicle_id: boat,
            root_vehicle_id: boat,
        })
    );
    assert!(store.get(boat).is_some());
    assert_eq!(store.passengers(boat), &[second_player]);
    assert_eq!(store.external_player_vehicle(first_player), None);
    assert_eq!(store.external_player_vehicle(second_player), Some(boat));
    assert!(write_entity_chunk(&store, (0, 0)).unwrap().is_empty());

    assert_eq!(
        store.unload_external_player_vehicle(second_player),
        Some(
            crate::entity::ExternalPlayerVehicleUnload::UnloadedRootTree {
                direct_vehicle_id: boat,
                root_vehicle_id: boat,
                entity_ids: vec![boat],
            }
        )
    );
    assert!(store.is_empty());
    assert!(store.drain_lifecycle_events().iter().any(|event| matches!(
        event,
        EntityLifecycleEvent::Removed { entity, .. }
            if entity.id() == boat
                && entity.removal_reason() == Some(RemovalReason::UnloadedWithPlayer)
    )));

    let mut nested = EntityStore::new(0x4558_5456, unoccupied_ids(&native));
    let root = crate::entity::spawn_minecart(&mut nested, position(), 0.0).unwrap();
    let child = crate::entity::spawn_boat(
        &mut nested,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        position(),
        0.0,
    )
    .unwrap();
    assert!(nested.link_existing_passenger(root, child));
    assert!(nested.attach_test_player(
        players[2],
        child,
        crate::entity::ExternalPlayerCanRide::Allowed
    ));
    nested.drain_lifecycle_events();
    assert_eq!(
        nested.unload_external_player_vehicle(players[2]),
        Some(
            crate::entity::ExternalPlayerVehicleUnload::UnloadedRootTree {
                direct_vehicle_id: child,
                root_vehicle_id: root,
                entity_ids: vec![child, root],
            }
        )
    );
    assert!(nested.is_empty());
    let removed_ids = nested
        .drain_lifecycle_events()
        .into_iter()
        .filter_map(|event| match event {
            EntityLifecycleEvent::Removed { entity, .. } => Some(entity.id()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(removed_ids, vec![child, root]);
}

#[test]
fn external_player_vehicle_root_save_load_is_not_region_duplicated() {
    let (mut native, players) = joined_players(1);
    let mut source = EntityStore::new(0x4558_5453, unoccupied_ids(&native));
    let chest = crate::entity::spawn_chest_boat(
        &mut source,
        VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
        position(),
        0.0,
    )
    .unwrap();
    let stored_passenger = crate::entity::spawn_armor_stand(&mut source, position()).unwrap();
    assert!(link_passenger(&mut source, chest, stored_passenger));
    let attach_uuid = source.get(chest).unwrap().uuid();
    let source_chunk = write_entity_chunk(&source, (0, 0)).unwrap();
    let source_record = source_chunk.entities().next().unwrap();
    let player_id = players[0];
    let mut store = EntityStore::new(0x4558_5454, unoccupied_ids(&native));
    let detached_vehicle = crate::entity::spawn_minecart(&mut store, position(), 0.0).unwrap();
    assert!(store.attach_test_player(
        player_id,
        detached_vehicle,
        crate::entity::ExternalPlayerCanRide::Allowed,
    ));
    assert_eq!(
        store.detach_external_player(player_id),
        Some(detached_vehicle)
    );
    store
        .remove(detached_vehicle, RemovalReason::Discarded)
        .unwrap();
    assert!(store.drain_lifecycle_events().iter().any(|event| matches!(
        event,
        EntityLifecycleEvent::ExternalPlayerBoardingCooldownReset {
            player_entity_id: signal_player_id, ..
        } if *signal_player_id == player_id
    )));
    let root = crate::entity::read_external_player_root_vehicle(
        &mut store,
        source_record,
        attach_uuid,
        player_id,
    )
    .expect("forced RootVehicle relation restores over normal capacity");
    let restored_stored = store
        .passengers(root)
        .iter()
        .copied()
        .find(|id| *id != player_id)
        .unwrap();
    assert_eq!(store.passengers(root), &[player_id, restored_stored]);
    let (attach_uuid, record) =
        crate::entity::write_external_player_root_vehicle(&store, player_id)
            .unwrap()
            .expect("the sole external player root is saveable");
    assert_eq!(attach_uuid, store.get(root).unwrap().uuid());
    assert_eq!(record.as_ref().passengers().len(), 1);
    assert!(write_entity_chunk(&store, (0, 0)).unwrap().is_empty());

    let mut restored = EntityStore::new(0x4558_5452, unoccupied_ids(&native));
    restored.place_test_player(player_id, position());
    let root = crate::entity::read_external_player_root_vehicle(
        &mut restored,
        record.as_ref(),
        attach_uuid,
        player_id,
    )
    .expect("forced RootVehicle relation restores");
    assert_eq!(restored.external_player_vehicle(player_id), Some(root));
    let restored_stored = restored
        .passengers(root)
        .iter()
        .copied()
        .find(|id| *id != player_id)
        .unwrap();
    assert_eq!(restored.passengers(root), &[player_id, restored_stored]);
    assert!(restored
        .external_player_vehicle_placement(player_id, 0.0)
        .is_some());
    assert!(restored.get(player_id).is_none());
    native.with_level(|ctx| tick_entities(&mut restored, ctx, true));
    assert_eq!(restored.external_player_vehicle(player_id), Some(root));
    let (roundtrip_attach, roundtrip_record) =
        crate::entity::write_external_player_root_vehicle(&restored, player_id)
            .unwrap()
            .unwrap();
    assert_eq!(roundtrip_attach, attach_uuid);
    assert_eq!(roundtrip_record.as_ref().uuid(), record.as_ref().uuid());
    assert_eq!(roundtrip_record.as_ref().passengers().len(), 1);
}

#[test]
fn external_player_vehicle_root_restore_uses_live_registry_tags() {
    let (native, players) = joined_players(2);
    let mut source = EntityStore::new(0x4558_5449, unoccupied_ids(&native));
    let falling = source.spawn(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        position(),
        EntityData::FallingBlock(FallingBlockData::new(
            state_id("minecraft:sand"),
            pos(0, 64, 0),
        )),
    );
    let attach_uuid = source.get(falling).unwrap().uuid();
    let saved = write_entity_chunk(&source, (0, 0)).unwrap();
    let mut raw = saved.entities().next().unwrap().raw().to_vec();
    raw.retain(|entry| entry.name != "HurtEntities");
    let record = bbb_server_persistence::Java26EntityRecord::parse(&raw).unwrap();

    let tagged =
        registry_tag_snapshot(&[("minecraft:block", "minecraft:anvil", &["minecraft:sand"])]);
    let mut restored = EntityStore::new(0x4558_5451, unoccupied_ids(&native));
    restored.place_test_player(players[0], position());
    let root = crate::entity::read_external_player_root_vehicle_with_registry_tags(
        &mut restored,
        record.as_ref(),
        attach_uuid,
        players[0],
        &tagged,
    )
    .unwrap();
    assert!(falling_block_state(&restored, root).hurt_entities());

    let untagged = registry_tag_snapshot(&[("minecraft:block", "minecraft:anvil", &[])]);
    let mut untagged_store = EntityStore::new(0x4558_5452, unoccupied_ids(&native));
    untagged_store.place_test_player(players[1], position());
    let untagged_root = crate::entity::read_external_player_root_vehicle_with_registry_tags(
        &mut untagged_store,
        record.as_ref(),
        attach_uuid,
        players[1],
        &untagged,
    )
    .unwrap();
    assert!(!falling_block_state(&untagged_store, untagged_root).hurt_entities());
}

#[test]
fn boat_subtypes_accept_empty_generated_attachments_and_enforce_capacity() {
    assert!(vanilla_entity_type_passenger_attachments(VANILLA_ENTITY_TYPE_OAK_BOAT_ID).is_none());
    let mut store = EntityStore::new(0x424f_4154, EntityIdCounter::new());
    let boat =
        crate::entity::spawn_boat(&mut store, VANILLA_ENTITY_TYPE_OAK_BOAT_ID, position(), 0.0)
            .unwrap();
    let first = crate::entity::spawn_armor_stand(&mut store, position()).unwrap();
    let second = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_PIG_ID, position())
        .unwrap();
    let third = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, position())
        .unwrap();
    assert!(link_passenger(&mut store, boat, first));
    assert!(link_passenger(&mut store, boat, second));
    assert!(!link_passenger(&mut store, boat, third));
    assert_eq!(store.passengers(boat), &[first, second]);
    assert_eq!(store.vehicle(third), None);

    let chest = crate::entity::spawn_chest_boat(
        &mut store,
        VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
        Vec3d {
            x: 10.5,
            ..position()
        },
        0.0,
    )
    .unwrap();
    assert!(link_passenger(&mut store, chest, third));
    let extra = crate::entity::spawn_armor_stand(&mut store, position()).unwrap();
    assert!(!link_passenger(&mut store, chest, extra));
    assert_eq!(store.passengers(chest), &[third]);
    assert_eq!(store.vehicle(extra), None);
}

#[test]
fn normal_generated_and_minecart_admission_refuses_a_second_passenger() {
    let mut store = EntityStore::new(0x4f4e_4521, EntityIdCounter::new());
    let minecart = crate::entity::spawn_minecart(&mut store, position(), 0.0).unwrap();
    let first = crate::entity::spawn_armor_stand(&mut store, position()).unwrap();
    let second = crate::entity::spawn_armor_stand(&mut store, position()).unwrap();
    assert!(store.link_existing_passenger(minecart, first));
    assert!(!store.link_existing_passenger(minecart, second));
    assert_eq!(store.passengers(minecart), &[first]);

    let chicken = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_CHICKEN_ID, position())
        .unwrap();
    assert!(store.link_existing_passenger(chicken, second));
    let extra = crate::entity::spawn_armor_stand(&mut store, position()).unwrap();
    assert!(!store.link_existing_passenger(chicken, extra));
    assert_eq!(store.passengers(chicken), &[second]);
    assert_eq!(store.vehicle(extra), None);
}

#[test]
fn forced_over_capacity_boat_places_its_saved_passengers() {
    let mut store = EntityStore::new(0x5354_4147, EntityIdCounter::new());
    let entities = [
        "minecraft:oak_boat",
        "minecraft:armor_stand",
        "minecraft:pig",
        "minecraft:cow",
    ]
    .into_iter()
    .map(|resource_id| {
        let entity =
            crate::entity::save::create_fresh_entity(&mut store, resource_id, position()).unwrap();
        store.add_fresh(entity)
    })
    .collect::<Vec<_>>();
    let ids = entities;
    for passenger_id in &ids[1..] {
        assert!(store.link_existing_passenger_forced(ids[0], *passenger_id));
    }
    assert_eq!(store.passengers(ids[0]), &ids[1..]);
    for passenger_id in &ids[1..] {
        assert_eq!(store.vehicle(*passenger_id), Some(ids[0]));
    }

    NativeLevel::new().with_level(|ctx| tick_entities(&mut store, ctx, true));

    assert_eq!(store.passengers(ids[0]), &ids[1..]);
    let boat_z = store.get(ids[0]).unwrap().position().z;
    assert_f64_close(store.get(ids[1]).unwrap().position().z - boat_z, 0.2);
    assert_f64_close(store.get(ids[2]).unwrap().position().z - boat_z, -0.4);
    assert_f64_close(store.get(ids[3]).unwrap().position().z - boat_z, -0.4);
}

#[test]
fn saved_over_capacity_chest_boat_tree_restores_forced_relations() {
    let mut store = EntityStore::new(0x5341_5645, EntityIdCounter::new());
    let chest = crate::entity::spawn_chest_boat(
        &mut store,
        VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
        position(),
        0.0,
    )
    .unwrap();
    let first = crate::entity::spawn_armor_stand(&mut store, position()).unwrap();
    let second = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_PIG_ID, position())
        .unwrap();
    assert!(link_passenger(&mut store, chest, first));
    assert!(store.link_existing_passenger_forced(chest, second));
    let document = write_entity_chunk(&store, (0, 0)).expect("forced tree writes canonically");

    let mut restored = EntityStore::new(0x5245_5354, EntityIdCounter::new());
    let roots = read_entity_chunk(&mut restored, &document).expect("forced tree restores");
    assert_eq!(roots.len(), 1);
    let restored_chest = roots[0];
    let restored_passengers = restored.passengers(restored_chest).to_vec();
    assert_eq!(restored_passengers.len(), 2);
    assert_eq!(
        restored.vehicle(restored_passengers[0]),
        Some(restored_chest)
    );
    assert_eq!(
        restored.vehicle(restored_passengers[1]),
        Some(restored_chest)
    );

    NativeLevel::new().with_level(|ctx| tick_entities(&mut restored, ctx, true));

    assert_eq!(restored.passengers(restored_chest), restored_passengers);
}

#[test]
fn boat_and_raft_ride_heights_and_offsets_use_live_subtype_geometry() {
    let mut store = EntityStore::new(0x4845_4947, EntityIdCounter::new());
    let boat =
        crate::entity::spawn_boat(&mut store, VANILLA_ENTITY_TYPE_OAK_BOAT_ID, position(), 0.0)
            .unwrap();
    let boat_rider = crate::entity::spawn_armor_stand(&mut store, position()).unwrap();
    assert!(link_passenger(&mut store, boat, boat_rider));

    let chest_position = Vec3d {
        x: 10.5,
        ..position()
    };
    let chest = crate::entity::spawn_chest_boat(
        &mut store,
        VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
        chest_position,
        0.0,
    )
    .unwrap();
    let chest_rider = crate::entity::spawn_armor_stand(&mut store, chest_position).unwrap();
    assert!(link_passenger(&mut store, chest, chest_rider));

    let raft_position = Vec3d {
        x: 12.5,
        ..position()
    };
    let raft = crate::entity::spawn_boat(
        &mut store,
        VANILLA_ENTITY_TYPE_BAMBOO_RAFT_ID,
        raft_position,
        0.0,
    )
    .unwrap();
    let raft_rider = crate::entity::spawn_armor_stand(&mut store, raft_position).unwrap();
    assert!(link_passenger(&mut store, raft, raft_rider));

    NativeLevel::new().with_level(|ctx| tick_entities(&mut store, ctx, true));

    let boat_entity = store.get(boat).unwrap();
    let boat_rider_entity = store.get(boat_rider).unwrap();
    assert_f64_close(
        boat_rider_entity.position().y - boat_entity.position().y,
        f64::from(boat_entity.bb_height() / 3.0),
    );
    assert_f64_close(
        boat_rider_entity.position().z - boat_entity.position().z,
        0.0,
    );

    let chest_entity = store.get(chest).unwrap();
    let chest_rider_entity = store.get(chest_rider).unwrap();
    assert_f64_close(
        chest_rider_entity.position().y - chest_entity.position().y,
        f64::from(chest_entity.bb_height() / 3.0),
    );
    assert_f64_close(
        chest_rider_entity.position().z - chest_entity.position().z,
        0.15,
    );

    let raft_entity = store.get(raft).unwrap();
    let raft_rider_entity = store.get(raft_rider).unwrap();
    assert_f64_close(
        raft_rider_entity.position().y - raft_entity.position().y,
        f64::from(raft_entity.bb_height() * 0.888_888_9),
    );
}

#[test]
fn full_boat_uses_ordered_offsets_and_animal_adjustment() {
    let mut store = EntityStore::new(0x4f46_4653, EntityIdCounter::new());
    let boat =
        crate::entity::spawn_boat(&mut store, VANILLA_ENTITY_TYPE_OAK_BOAT_ID, position(), 0.0)
            .unwrap();
    let first = crate::entity::spawn_armor_stand(&mut store, position()).unwrap();
    let animal = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, position())
        .unwrap();
    assert!(link_passenger(&mut store, boat, first));
    assert!(link_passenger(&mut store, boat, animal));

    NativeLevel::new().with_level(|ctx| tick_entities(&mut store, ctx, true));

    let boat_z = store.get(boat).unwrap().position().z;
    assert_f64_close(store.get(first).unwrap().position().z - boat_z, 0.2);
    let animal_entity = store.get(animal).unwrap();
    assert_f64_close(animal_entity.position().z - boat_z, -0.4);
    let rotation_offset = if animal % 2 == 0 { 90.0 } else { 270.0 };
    assert_f32_close(animal_entity.living().unwrap().y_body_rot, rotation_offset);
    assert_f32_close(
        animal_entity.living().unwrap().y_head_rot,
        animal_entity.y_rot() + rotation_offset,
    );
}

#[test]
fn merchant_minecart_passengers_use_the_lowered_attachment() {
    let mut store = EntityStore::new(0x4c4f_5745, EntityIdCounter::new());
    let villager_cart = crate::entity::spawn_minecart(&mut store, position(), 0.0).unwrap();
    let villager = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_VILLAGER_ID, position())
        .unwrap();
    let trader_position = Vec3d {
        x: 10.5,
        ..position()
    };
    let trader_cart = crate::entity::spawn_minecart(&mut store, trader_position, 0.0).unwrap();
    let trader = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID, trader_position)
        .unwrap();
    assert!(store.link_existing_passenger(villager_cart, villager));
    assert!(store.link_existing_passenger(trader_cart, trader));
    assert_eq!(
        vanilla_entity_type_passenger_attachments(
            store.get(villager_cart).unwrap().entity_type_id()
        )
        .unwrap()[0][1],
        0.1875
    );

    NativeLevel::new().with_level(|ctx| tick_entities(&mut store, ctx, true));

    assert_f64_close(
        store.get(villager).unwrap().position().y,
        store.get(villager_cart).unwrap().position().y,
    );
    assert_f64_close(
        store.get(trader).unwrap().position().y,
        store.get(trader_cart).unwrap().position().y,
    );
}

#[test]
fn boat_rotation_clamps_living_riders_but_leaves_breeze_rotation_untouched() {
    let mut store = EntityStore::new(0x524f_5441, EntityIdCounter::new());
    let boat =
        crate::entity::spawn_boat(&mut store, VANILLA_ENTITY_TYPE_OAK_BOAT_ID, position(), 0.0)
            .unwrap();
    let pig = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_PIG_ID, position())
        .unwrap();
    store.get_mut(pig).unwrap().set_rot(170.0, 0.0);
    assert!(link_passenger(&mut store, boat, pig));

    let breeze_position = Vec3d {
        x: 10.5,
        ..position()
    };
    let breeze_boat = crate::entity::spawn_boat(
        &mut store,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        breeze_position,
        0.0,
    )
    .unwrap();
    let breeze = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_BREEZE_ID, breeze_position)
        .unwrap();
    store.get_mut(breeze).unwrap().set_rot(170.0, 0.0);
    assert!(link_passenger(&mut store, breeze_boat, breeze));

    NativeLevel::new().with_level(|ctx| tick_entities(&mut store, ctx, true));

    let pig_entity = store.get(pig).unwrap();
    assert_f32_close(pig_entity.y_rot(), 105.0);
    assert_f32_close(pig_entity.living().unwrap().y_body_rot, 0.0);
    assert_f32_close(pig_entity.living().unwrap().y_head_rot, 105.0);
    assert!(crate::entity::living::wrap_degrees(store.get(breeze).unwrap().y_rot()).abs() > 105.0);
}

#[test]
fn ordinary_boat_rider_consumes_live_delta_rotation() {
    let mut store = EntityStore::new(0x4445_4c54, EntityIdCounter::new());
    let boat =
        crate::entity::spawn_boat(&mut store, VANILLA_ENTITY_TYPE_OAK_BOAT_ID, position(), 0.0)
            .unwrap();
    assert!(crate::entity::boat::set_delta_rotation_for_test(
        store.get_mut(boat).unwrap(),
        20.0,
    ));
    let rider = crate::entity::spawn_armor_stand(&mut store, position()).unwrap();
    let rider_entity = store.get_mut(rider).unwrap();
    rider_entity.set_rot(0.0, 0.0);
    rider_entity.living_mut().unwrap().y_body_rot = 0.0;
    rider_entity.living_mut().unwrap().y_head_rot = 0.0;
    assert!(link_passenger(&mut store, boat, rider));

    NativeLevel::new().with_level(|ctx| tick_entities(&mut store, ctx, true));

    let delta = crate::entity::boat::boat(store.get(boat).unwrap().data())
        .unwrap()
        .delta_rotation();
    assert_f32_close(store.get(rider).unwrap().y_rot(), delta);
    assert_f32_close(store.get(rider).unwrap().living().unwrap().y_body_rot, 0.0);
    assert_f32_close(
        store.get(rider).unwrap().living().unwrap().y_head_rot,
        delta,
    );
}

#[test]
fn passenger_tree_uses_rotated_attachments() {
    let mut store = EntityStore::new(0x5041_5353, EntityIdCounter::new());
    let root = crate::entity::spawn_minecart(&mut store, position(), 0.0).unwrap();
    let chicken = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_CHICKEN_ID, position())
        .unwrap();
    let pig = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_PIG_ID, position())
        .unwrap();
    store.get_mut(chicken).unwrap().set_rot(90.0, 0.0);
    assert!(store.link_existing_passenger(root, chicken));
    assert!(store.link_existing_passenger(chicken, pig));

    NativeLevel::new().with_level(|ctx| tick_entities(&mut store, ctx, true));
    let chicken_entity = store.get(chicken).unwrap();
    let pig_entity = store.get(pig).unwrap();
    let point =
        vanilla_entity_type_passenger_attachments(chicken_entity.entity_type_id()).unwrap()[0];
    let vehicle_offset = rotated(point, chicken_entity.y_rot());
    let rider_offset = rotated(
        vanilla_entity_type_vehicle_attachment(pig_entity.entity_type_id()).unwrap(),
        pig_entity.y_rot(),
    );
    assert_eq!(
        pig_entity.position(),
        Vec3d {
            x: chicken_entity.position().x + vehicle_offset.x - rider_offset.x,
            y: chicken_entity.position().y + vehicle_offset.y - rider_offset.y,
            z: chicken_entity.position().z + vehicle_offset.z - rider_offset.z,
        }
    );
}

#[test]
fn removed_vehicle_ejects_its_passenger_with_a_boarding_cooldown() {
    let mut store = EntityStore::new(0x454a_4543, EntityIdCounter::new());
    let vehicle = crate::entity::spawn_minecart(&mut store, position(), 0.0).unwrap();
    let passenger = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_PIG_ID, position())
        .unwrap();
    assert!(store.link_existing_passenger(vehicle, passenger));
    store.drain_lifecycle_events();

    store.remove(vehicle, RemovalReason::Killed).unwrap();
    assert_eq!(store.vehicle(passenger), None);
    assert_eq!(store.get(passenger).unwrap().boarding_cooldown(), 60);
    let replacement = crate::entity::spawn_minecart(&mut store, position(), 0.0).unwrap();
    assert!(!store.link_existing_passenger(replacement, passenger));
    store.drain_lifecycle_events();

    NativeLevel::new().with_level(|ctx| tick_entities(&mut store, ctx, true));
    assert_eq!(store.get(passenger).unwrap().boarding_cooldown(), 59);
}

#[test]
fn a_mounted_hostile_mob_despawns_far_from_the_live_player_only_after_dismounting() {
    let (mut level, _) = joined_players(1);
    level.world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 500.5,
            y: 64.0,
            z: 0.5,
        },
    );
    let mut store = EntityStore::new(0x4453_504e, unoccupied_ids(&level));
    let vehicle = crate::entity::spawn_minecart(&mut store, position(), 0.0).unwrap();
    let passenger = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, position())
        .unwrap();
    assert!(store.link_existing_passenger(vehicle, passenger));
    store.set_difficulty(Difficulty::Normal);
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    assert!(store.get(passenger).is_some());
    assert_eq!(store.vehicle(passenger), Some(vehicle));
    assert_eq!(store.passengers(vehicle), &[passenger]);
    assert!(store.stop_stored_riding(passenger));
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    assert!(store.get(passenger).is_none());
    assert!(store.get(vehicle).is_some());
    assert!(store.passengers(vehicle).is_empty());
}

#[test]
fn an_unloaded_root_keeps_its_passenger_tree_and_positions() {
    let mut native = NativeLevel::new();
    let mut store = EntityStore::new(0x5241_4e47, EntityIdCounter::new());
    let unloaded = Vec3d {
        x: 1000.5,
        ..position()
    };
    let root = crate::entity::spawn_minecart(&mut store, unloaded, 0.0).unwrap();
    let passenger = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_PIG_ID, unloaded)
        .unwrap();
    assert!(store.link_existing_passenger(root, passenger));
    native.with_level(|ctx| tick_entities(&mut store, ctx, true));
    assert_eq!(store.get(root).unwrap().position(), unloaded);
    assert_eq!(store.get(passenger).unwrap().position(), unloaded);
    assert_eq!(store.vehicle(passenger), Some(root));
    assert_eq!(store.passengers(root), &[passenger]);
}

#[test]
fn a_riding_passenger_cannot_link_a_second_vehicle() {
    let mut store = EntityStore::new(0x5354_414c, EntityIdCounter::new());
    let first = crate::entity::spawn_minecart(&mut store, position(), 0.0).unwrap();
    let second = crate::entity::spawn_minecart(&mut store, position(), 0.0).unwrap();
    let passenger = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_PIG_ID, position())
        .unwrap();
    assert!(store.link_existing_passenger(first, passenger));
    assert!(!store.link_existing_passenger(second, passenger));
    assert_eq!(store.vehicle(passenger), Some(first));
    assert_eq!(store.passengers(first), &[passenger]);
    assert!(store.passengers(second).is_empty());
}
