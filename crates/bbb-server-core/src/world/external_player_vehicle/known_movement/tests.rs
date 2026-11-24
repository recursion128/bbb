use super::*;
use crate::entity::{ExternalPlayerCanRide, RemovalReason};
use bbb_protocol::entity_types::*;
use bbb_protocol::packets::BlockPos;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::save::Java26BuiltinDimension;
use bbb_server_types::stats::custom_stat;

use crate::mailbox::{world_command_mailbox, WorldMailboxLimits};
use crate::world::ServerWorldConfig;

const ALICE: ConnectionId = ConnectionId(1);
const BOB: ConnectionId = ConnectionId(2);
const RAW: Vec3d = Vec3d {
    x: 0.01,
    y: -0.02,
    z: 0.03,
};
const VEHICLE: Vec3d = Vec3d {
    x: -0.1,
    y: 0.2,
    z: -0.3,
};

fn world() -> ServerWorld {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    crate::tests::join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    world.levels[ALICE].known_movement = RAW;
    world.levels[BOB].known_movement = Vec3d::default();
    world
}

fn entities(world: &mut ServerWorld) -> &mut EntityStore {
    &mut world.level_mut(Java26BuiltinDimension::Overworld).entities
}

fn boat(world: &mut ServerWorld, kind: i32) -> i32 {
    let position = world.levels[ALICE].position();
    let store = entities(world);
    let id = crate::entity::spawn_boat(store, kind, position, 0.0)
        .or_else(|| crate::entity::spawn_chest_boat(store, kind, position, 0.0))
        .unwrap();
    store.get_mut(id).unwrap().set_delta_movement(VEHICLE);
    id
}

fn mount(world: &mut ServerWorld, player: ConnectionId, vehicle: i32) {
    let player = world.levels[player].entity_id();
    assert!(entities(world).attach_test_player(player, vehicle, ExternalPlayerCanRide::Allowed,));
}

#[test]
fn known_movement_unmounted_and_boat_controller_use_client_value() {
    let mut world = world();
    assert_eq!(world.player_known_movement(ALICE), Some(RAW));
    assert_eq!(world.player_known_movement(ConnectionId(999)), None);
    for kind in [
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        VANILLA_ENTITY_TYPE_BAMBOO_RAFT_ID,
        VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
        VANILLA_ENTITY_TYPE_BAMBOO_CHEST_RAFT_ID,
    ] {
        let boat = boat(&mut world, kind);
        mount(&mut world, ALICE, boat);
        assert_eq!(world.player_known_movement(ALICE), Some(RAW));
        let player_id = world.levels[ALICE].entity_id();
        assert_eq!(
            entities(&mut world).detach_external_player(player_id),
            Some(boat)
        );
    }
}

#[test]
fn known_movement_boat_rear_passenger_reads_controller_and_dead_vehicle_delta() {
    let mut world = world();
    let boat = boat(&mut world, VANILLA_ENTITY_TYPE_OAK_BOAT_ID);
    mount(&mut world, ALICE, boat);
    mount(&mut world, BOB, boat);
    assert_eq!(world.player_known_movement(BOB), Some(RAW));
    entities(&mut world)
        .get_mut(boat)
        .unwrap()
        .set_removed(RemovalReason::Killed);
    assert_eq!(world.player_known_movement(BOB), Some(VEHICLE));
    assert_eq!(world.player_known_movement(ALICE), Some(RAW));
}

#[test]
fn known_movement_minecart_uses_own_delta_with_old_behavior_clamp_and_nan() {
    let mut world = world();
    let position = world.levels[ALICE].position();
    let cart =
        crate::entity::minecart::spawn_minecart(entities(&mut world), position, 0.0).unwrap();
    mount(&mut world, ALICE, cart);
    for (movement, expected) in [
        (
            Vec3d {
                x: 3.0,
                y: 0.75,
                z: -4.0,
            },
            Vec3d {
                x: 0.4,
                y: 0.75,
                z: -0.4,
            },
        ),
        (
            Vec3d {
                x: f64::INFINITY,
                y: f64::NEG_INFINITY,
                z: -0.02,
            },
            Vec3d {
                x: 0.4,
                y: f64::NEG_INFINITY,
                z: -0.02,
            },
        ),
        (
            Vec3d {
                x: f64::NAN,
                ..VEHICLE
            },
            Vec3d::default(),
        ),
        (
            Vec3d {
                y: f64::NAN,
                ..VEHICLE
            },
            Vec3d::default(),
        ),
        (
            Vec3d {
                z: f64::NAN,
                ..VEHICLE
            },
            Vec3d::default(),
        ),
    ] {
        entities(&mut world)
            .get_mut(cart)
            .unwrap()
            .set_delta_movement(movement);
        assert_eq!(world.player_known_movement(ALICE), Some(expected));
    }
}

#[test]
fn known_movement_ordinary_and_unequipped_mobs_use_vehicle_delta() {
    let mut world = world();
    let position = world.levels[ALICE].position();
    for kind in [
        VANILLA_ENTITY_TYPE_COW_ID,
        VANILLA_ENTITY_TYPE_HORSE_ID,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
        VANILLA_ENTITY_TYPE_PIG_ID,
        VANILLA_ENTITY_TYPE_STRIDER_ID,
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
    ] {
        let vehicle = entities(&mut world)
            .spawn_general_living(kind, position)
            .unwrap();
        let player_id = world.levels[ALICE].entity_id();
        let store = entities(&mut world);
        store.get_mut(vehicle).unwrap().set_delta_movement(VEHICLE);
        assert!(store.attach_test_player(player_id, vehicle, ExternalPlayerCanRide::Allowed));
        assert_eq!(world.player_known_movement(ALICE), Some(VEHICLE));
        assert_eq!(
            entities(&mut world).detach_external_player(player_id),
            Some(vehicle)
        );
    }
}

mod controllers;

fn spawn_tree(world: &mut ServerWorld, resource: &str, passenger: &str) -> i32 {
    let position = world.levels[ALICE].position();
    let request = crate::entity::SpawnRequest {
        entity_type_id: vanilla_entity_type_for_resource_id(resource).unwrap().id,
        position,
        spawn_data: crate::entity::SpawnerSpawnData::new(
            vec![
                NbtSummaryEntry {
                    name: "id".into(),
                    value: NbtSummaryValue::String(resource.into()),
                },
                NbtSummaryEntry {
                    name: "Passengers".into(),
                    value: NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                        NbtSummaryEntry {
                            name: "id".into(),
                            value: NbtSummaryValue::String(passenger.into()),
                        },
                    ])]),
                },
            ],
            None,
            None,
        ),
        spawner_position: BlockPos { x: 0, y: 64, z: 0 },
        spawn_range: 4,
        max_nearby_entities: 6,
    };
    let plan = crate::entity::prepare_spawn_request(&request).unwrap();
    let store = entities(world);
    crate::entity::spawn_entity_tree(store, &plan).unwrap()
}

#[test]
fn known_movement_boat_with_first_mob_uses_delta() {
    let mut world = world();
    let boat = spawn_tree(&mut world, "minecraft:oak_boat", "minecraft:cow");
    entities(&mut world)
        .get_mut(boat)
        .unwrap()
        .set_delta_movement(VEHICLE);
    let store = &world.level(Java26BuiltinDimension::Overworld).entities;
    // Entity.java:2419-2435 inserts a joining Player ahead of the mob, so query
    // the vehicle getter directly while its stored first passenger remains first.
    assert_eq!(
        ServerWorld::known_movement_from_vehicle(
            EntityBodyReadView::new(store, None, None),
            store.get(boat).unwrap(),
            Some(world.levels.player_at(ALICE)),
            &crate::entity::living::valid_equipment::EquipmentRegistry(
                &EntityRegistryTagView::Builtin26_1
            ),
        ),
        Some(VEHICLE)
    );
}

#[test]
fn known_movement_uses_direct_vehicle_not_the_root_vehicle() {
    let mut world = world();
    let root = spawn_tree(&mut world, "minecraft:minecart", "minecraft:oak_boat");
    let boat = entities(&mut world).passengers(root)[0];
    mount(&mut world, ALICE, boat);
    mount(&mut world, BOB, boat);
    entities(&mut world)
        .get_mut(root)
        .unwrap()
        .set_delta_movement(VEHICLE);
    assert_eq!(world.player_known_movement(BOB), Some(RAW));
}

#[test]
fn known_movement_vehicle_sync_and_tick_keep_client_value_and_riding_stat() {
    let mut world = world();
    let boat = boat(&mut world, VANILLA_ENTITY_TYPE_OAK_BOAT_ID);
    mount(&mut world, ALICE, boat);
    let dimension = world.levels[ALICE].dimension;
    world.sync_external_players_after_vehicle_tick(&dimension, boat, &mut Vec::new());
    let stat = custom_stat("minecraft:boat_one_cm");
    let before = world.levels[ALICE].cold.stats.value(stat);
    let vehicle = entities(&mut world).get_mut(boat).unwrap();
    let mut position = vehicle.position();
    position.x += 1.0;
    vehicle.set_pos(position);
    world.sync_external_players_after_vehicle_tick(&dimension, boat, &mut Vec::new());
    assert_eq!(world.levels[ALICE].known_movement, RAW);
    assert_eq!(world.levels[ALICE].cold.stats.value(stat) - before, 100);
    world.tick(Vec::new());
    assert_eq!(world.levels[ALICE].known_movement, RAW);
}
