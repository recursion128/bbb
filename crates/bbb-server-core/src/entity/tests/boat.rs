use super::*;
use crate::connection::ConnectionId;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};

use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_ABSTRACT_BOAT_BUBBLE_TIME_DATA_ID, VANILLA_ABSTRACT_BOAT_PADDLE_LEFT_DATA_ID,
    VANILLA_ABSTRACT_BOAT_PADDLE_RIGHT_DATA_ID, VANILLA_VEHICLE_DAMAGE_DATA_ID,
    VANILLA_VEHICLE_HURT_DIR_DATA_ID, VANILLA_VEHICLE_HURT_TIME_DATA_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID;
use bbb_protocol::NbtSummaryEntry;

use crate::entity::boat::{self, BoatStatus, BUBBLE_TIME, DAMAGE_TO_DESTROY, DEFAULT_GRAVITY};
use crate::entity::damage::StoredDamageCausingEntity;
use crate::entity::{spawn_boat, BoatData};

/// A wooden boat and the raft, each beside the item its `getDropItem`
/// supplier names (`EntityType.java:196-253`).
const BOAT_TYPES: [&str; 2] = ["minecraft:oak_boat", "minecraft:bamboo_raft"];

fn joined_driver(level: &mut NativeLevel) -> i32 {
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut level.world, 1, "alice", 1);
    level.world.player_entity_id_for_test(ConnectionId(1))
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

fn oak_boat_with_ids(position: Vec3d, ids: EntityIdCounter) -> (EntityStore, i32) {
    let mut store = EntityStore::new(0x0b_0a_71, ids);
    let id = spawn_boat(&mut store, VANILLA_ENTITY_TYPE_OAK_BOAT_ID, position, 90.0).unwrap();
    (store, id)
}

fn native_boat(resource_id: &str) -> (NativeLevel, i32) {
    let mut level = NativeLevel::new();
    let kind = vanilla_entity_type_id_for_resource_id(resource_id).unwrap();
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = spawn_boat(
        &mut store,
        kind,
        Vec3d {
            x: 0.5,
            y: 70.0,
            z: 0.5,
        },
        0.0,
    )
    .unwrap();
    let data = store.get(id).unwrap().data().clone();
    let (id, _) = level.world.spawn_stored_entity_for_test(
        DIMENSION,
        kind,
        Vec3d {
            x: 0.5,
            y: 70.0,
            z: 0.5,
        },
        data,
    );
    (level, id)
}

fn boat_data(store: &EntityStore, id: i32) -> BoatData {
    *boat::boat(store.get(id).expect("entity is live").data()).expect("entity is a boat")
}

/// One oak boat standing at `position` in a fresh store.
fn oak_boat_at(position: Vec3d) -> (EntityStore, i32) {
    let mut store = EntityStore::new(0x0b_0a_71, EntityIdCounter::new());
    let id = spawn_boat(&mut store, VANILLA_ENTITY_TYPE_OAK_BOAT_ID, position, 90.0)
        .expect("a fresh oak boat");
    (store, id)
}

fn water_source() -> i32 {
    state_id("minecraft:water")
}

fn flowing_water() -> i32 {
    state_with("minecraft:water", &[("level", "1")])
}

fn submerged_level_at(position: Vec3d) -> NativeLevel {
    let center_x = position.x.floor() as i32;
    let center_y = position.y.floor() as i32;
    let center_z = position.z.floor() as i32;
    let mut level = NativeLevel::new();

    for x in center_x - 2..=center_x + 2 {
        for y in center_y..=center_y + 1 {
            for z in center_z - 2..=center_z + 2 {
                level
                    .world
                    .set_block(DIMENSION, BlockPos { x, y, z }, water_source(), 18);
            }
        }
    }
    level
}

fn attach_external_player_to_boat(store: &mut EntityStore, boat: i32, player: i32) {
    assert!(store.attach_test_player(player, boat, crate::entity::ExternalPlayerCanRide::Allowed,));
}

#[test]
fn every_boat_type_spawns_with_the_synched_defaults_and_the_placement_rotation() {
    for resource_id in BOAT_TYPES {
        let entity_type_id = vanilla_entity_type_id_for_resource_id(resource_id)
            .unwrap_or_else(|| panic!("{resource_id} is a built-in 26.1 entity type"));
        let mut store = EntityStore::new(0, EntityIdCounter::new());
        let id = spawn_boat(
            &mut store,
            entity_type_id,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
            123.5,
        )
        .unwrap_or_else(|| panic!("{resource_id} has a constructor"));
        let entity = store.get(id).expect("entity is live");
        assert_eq!(entity.entity_type_id(), entity_type_id);
        assert_eq!(entity.data().entity_resource_id(), resource_id);
        assert_eq!(entity.y_rot(), 123.5);
        assert_eq!(entity.x_rot(), 0.0);
        // `EntityType.java:200-202`: every boat is sized 1.375 x 0.5625.
        assert_eq!(entity.bb_width(), 1.375);
        assert_eq!(entity.bb_height(), 0.5625);
        // `AbstractBoat.getDefaultGravity()` (`AbstractBoat.java:525-526`).
        assert_eq!(entity.gravity(), DEFAULT_GRAVITY);
        // `AbstractBoat.isPickable()` (`:193-194`).
        assert!(entity.is_pickable());

        let data = boat::boat(entity.data()).expect("entity is a boat");
        assert_eq!(data.hurt_time(), 0);
        // `VehicleEntity.java:82`, the one non-zero synched default.
        assert_eq!(data.hurt_dir(), 1);
        assert_eq!(data.damage(), 0.0);
        assert_eq!(data.bubble_time(), 0);
        assert_eq!(data.paddle_state(), (false, false));
        assert_eq!(data.status(), None);
        // The drop item and the entity share a registry name.
        assert!(plain_stack(resource_id, 1).is_some());
    }
}

/// `AbstractBoat.getGroundFriction` (`:428-455`) divides by the number of
/// blocks it sampled, so an empty sample is a NaN that `getStatus`'s
/// `friction > 0.0F` reads as `IN_AIR` (`:381-386`).
#[test]
fn a_boat_over_nothing_is_in_air_and_falls_under_its_own_gravity() {
    let mut level = NativeLevel::new();

    let (mut store, id) = oak_boat_at(Vec3d {
        x: 0.5,
        y: 70.0,
        z: 0.5,
    });
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));

    let data = boat_data(&store, id);
    assert_eq!(data.status(), Some(BoatStatus::InAir));
    assert_eq!(data.out_of_control_ticks(), 0.0);
    let entity = store.get(id).expect("entity is live");
    assert_eq!(entity.delta_movement().y, -DEFAULT_GRAVITY);
    assert_eq!(entity.position().y, 70.0 - DEFAULT_GRAVITY);
    assert!(!entity.on_ground());
}

#[test]
fn an_airborne_boat_scales_its_horizontal_delta_by_the_in_air_friction() {
    let mut level = NativeLevel::new();

    let (mut store, id) = oak_boat_at(Vec3d {
        x: 0.5,
        y: 70.0,
        z: 0.5,
    });
    store
        .get_mut(id)
        .expect("entity is live")
        .set_delta_movement(Vec3d {
            x: 0.5,
            y: 0.0,
            z: -0.25,
        });
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));

    let entity = store.get(id).expect("entity is live");
    // `:562-564`: `movement.x * invFriction` with the float 0.9F widened.
    assert_eq!(entity.delta_movement().x, 0.5 * f64::from(0.9_f32));
    assert_eq!(entity.delta_movement().z, -0.25 * f64::from(0.9_f32));
}

#[test]
fn an_airborne_boat_keeps_its_move_across_both_inside_block_calls() {
    let (mut store, id) = oak_boat_at(Vec3d {
        x: 0.5,
        y: 70.0,
        z: 0.5,
    });
    let mut entity = store.detach(id).unwrap();
    entity.portal_cooldown = 1;
    entity.set_delta_movement(Vec3d {
        x: 3.0,
        y: 0.0,
        z: 0.0,
    });
    let entry = BlockPos { x: 2, y: 70, z: 0 };
    let mut level = NativeLevel::new();
    level
        .world
        .set_block(DIMENSION, entry, state_id("minecraft:nether_portal"), 18);

    assert!(level
        .with_level(|ctx| boat::tick(&mut entity, &mut store, ctx, false, false))
        .is_some());

    assert!(entity.position().x > 3.0);
    assert_eq!(entity.portal_cooldown(), 0);
    assert_eq!(entity.portal_process().unwrap().entry_position(), entry);
}

#[test]
fn a_boat_resting_on_a_block_is_on_land_and_takes_that_block_friction() {
    let stone = state_id("minecraft:stone");
    let mut level = NativeLevel::new();
    for x in -3..=3 {
        for z in -3..=3 {
            level
                .world
                .set_block(DIMENSION, BlockPos { x, y: 69, z }, stone, 18);
        }
    }
    let (mut store, id) = oak_boat_at(Vec3d {
        x: 0.5,
        y: 70.0,
        z: 0.5,
    });
    store
        .get_mut(id)
        .expect("entity is live")
        .set_delta_movement(Vec3d {
            x: 0.5,
            y: 0.0,
            z: 0.0,
        });
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));

    let data = boat_data(&store, id);
    assert_eq!(data.status(), Some(BoatStatus::OnLand));
    // `:381-384`: the sampled friction is stashed for `floatBoat`. The nine
    // interior columns of the sample are summed and divided in Java float
    // arithmetic, which does not land back on exactly `0.6F`.
    let mut summed = 0.0_f32;
    for _ in 0..9 {
        summed += 0.6;
    }
    assert_eq!(data.land_friction(), summed / 9.0);
    let entity = store.get(id).expect("entity is live");
    assert_eq!(
        entity.delta_movement().x,
        0.5 * f64::from(data.land_friction())
    );
    assert_eq!(entity.position().y, 70.0);
    assert!(entity.on_ground());
}

#[test]
fn a_boat_lying_in_a_water_surface_is_in_water_and_is_pushed_back_up() {
    let mut level = NativeLevel::new();
    for x in -3..=3 {
        for z in -3..=3 {
            level
                .world
                .set_block(DIMENSION, BlockPos { x, y: 69, z }, water_source(), 18);
        }
    }
    let position = Vec3d {
        x: 0.5,
        y: 69.5,
        z: 0.5,
    };
    let (mut store, id) = oak_boat_at(position);
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));

    let data = boat_data(&store, id);
    assert_eq!(data.status(), Some(BoatStatus::InWater));
    // `checkInWater` (`:464-483`) records the highest water surface it saw:
    // a source with no water above it is `getOwnHeight()`, 8/9 of a block.
    let expected_level = f64::from(69.0_f32 + 8.0 / 9.0);
    assert_eq!(data.water_level(), expected_level);

    // `:544-546` then `:565-568`: buoyancy over the boat's own height, scaled
    // by the *default* gravity rather than the current one.
    let buoyancy = (expected_level - position.y) / f64::from(0.5625_f32);
    let expected_y = (-DEFAULT_GRAVITY + buoyancy * (DEFAULT_GRAVITY / 0.65)) * 0.75;
    let entity = store.get(id).expect("entity is live");
    assert_eq!(entity.delta_movement().y, expected_y);
    assert!(expected_y > 0.0, "a submerged boat rises");
}

#[test]
fn water_over_the_boat_is_under_water_and_flowing_water_is_its_own_status() {
    for (block, expected) in [
        (water_source(), BoatStatus::UnderWater),
        (flowing_water(), BoatStatus::UnderFlowingWater),
    ] {
        let mut level = NativeLevel::new();
        for x in -3..=3 {
            for z in -3..=3 {
                level
                    .world
                    .set_block(DIMENSION, BlockPos { x, y: 69, z }, water_source(), 18);
            }
        }
        for x in -3..=3 {
            for z in -3..=3 {
                level
                    .world
                    .set_block(DIMENSION, BlockPos { x, y: 70, z }, block, 18);
            }
        }
        let (mut store, id) = oak_boat_at(Vec3d {
            x: 0.5,
            y: 69.5,
            z: 0.5,
        });
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
        let data = boat_data(&store, id);
        assert_eq!(data.status(), Some(expected));
        // `:211-215`: only the two submerged statuses accumulate.
        assert_eq!(data.out_of_control_ticks(), 1.0);
        // `:375-377`: a submerged boat's water level is its own box top.
        assert_eq!(data.water_level(), 69.5 + f64::from(0.5625_f32));

        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
        assert_eq!(boat_data(&store, id).out_of_control_ticks(), 2.0);
    }
}

#[test]
fn external_player_controlled_boat_keeps_client_authority_through_server_tick() {
    let mut level = NativeLevel::new();

    let position = Vec3d {
        x: 0.5,
        y: 70.0,
        z: 0.5,
    };
    let player = joined_driver(&mut level);
    let (mut store, boat) = oak_boat_with_ids(position, unoccupied_ids(&level));
    attach_external_player_to_boat(&mut store, boat, player);
    assert!(store.set_external_player_boat_paddles(player, true, false));
    store.get_mut(boat).unwrap().set_delta_movement(Vec3d {
        x: 0.5,
        y: 0.25,
        z: -0.125,
    });
    store.drain_lifecycle_events();

    level.with_level(|ctx| tick_entities(&mut store, ctx, true));

    let entity = store.get(boat).unwrap();
    let data = boat_data(&store, boat);
    assert_eq!(entity.position(), position);
    assert_eq!(entity.delta_movement(), Vec3d::default());
    assert_eq!(data.status(), Some(BoatStatus::InAir));
    assert_eq!(data.paddle_state(), (true, false));
    assert_eq!(store.external_player_vehicle(player), Some(boat));
    assert!(store
        .drain_lifecycle_events()
        .iter()
        .all(|event| !matches!(event, EntityLifecycleEvent::PassengersChanged { .. })));
}

#[test]
fn external_player_controlled_boat_absence_keeps_server_authority() {
    let mut level = NativeLevel::new();

    let position = Vec3d {
        x: 0.5,
        y: 70.0,
        z: 0.5,
    };
    let (mut store, boat) = oak_boat_at(position);
    let passenger = crate::entity::spawn_armor_stand(&mut store, position).unwrap();
    assert!(store.link_existing_passenger(boat, passenger));
    boat::set_paddle_state(store.get_mut(boat).unwrap(), true, true);
    store.get_mut(boat).unwrap().set_delta_movement(Vec3d {
        x: 0.5,
        y: 0.0,
        z: 0.0,
    });

    level.with_level(|ctx| tick_entities(&mut store, ctx, true));

    let entity = store.get(boat).unwrap();
    assert_ne!(entity.position(), position);
    assert_ne!(entity.delta_movement(), Vec3d::default());
    assert_eq!(boat_data(&store, boat).paddle_state(), (false, false));
    assert_eq!(store.vehicle(passenger), Some(boat));
    assert_eq!(store.passengers(boat), &[passenger]);
}

#[test]
fn external_player_controlled_boat_nested_under_vehicle_remains_client_authoritative() {
    let mut level = NativeLevel::new();
    let player = joined_driver(&mut level);
    let mut store = EntityStore::new(0x4e45_5354, unoccupied_ids(&level));
    let root = crate::entity::spawn_minecart(
        &mut store,
        Vec3d {
            y: 70.0,
            ..Vec3d::default()
        },
        0.0,
    )
    .unwrap();
    let boat = spawn_boat(
        &mut store,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        Vec3d::default(),
        0.0,
    )
    .unwrap();
    assert!(store.link_existing_passenger(root, boat));
    attach_external_player_to_boat(&mut store, boat, player);
    assert!(store.set_external_player_boat_paddles(player, true, true));
    store.drain_lifecycle_events();

    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    assert_eq!(store.get(boat).unwrap().delta_movement(), Vec3d::default());
    assert_eq!(boat_data(&store, boat).paddle_state(), (true, true));
    assert_eq!(store.external_player_vehicle(player), Some(boat));
    assert_eq!(store.external_player_root_vehicle(player), Some(root));
    assert!(store
        .drain_lifecycle_events()
        .iter()
        .all(|event| !matches!(event, EntityLifecycleEvent::PassengersChanged { .. })));
}

#[test]
fn external_player_controlled_boat_underwater_timeout_ejects_and_moves() {
    let mut level = NativeLevel::new();
    for x in -3..=3 {
        for z in -3..=3 {
            level
                .world
                .set_block(DIMENSION, BlockPos { x, y: 69, z }, water_source(), 18);
        }
    }
    for x in -3..=3 {
        for z in -3..=3 {
            level
                .world
                .set_block(DIMENSION, BlockPos { x, y: 70, z }, water_source(), 18);
        }
    }
    let position = Vec3d {
        x: 0.5,
        y: 69.5,
        z: 0.5,
    };
    let player = joined_driver(&mut level);
    let (mut store, boat) = oak_boat_with_ids(position, unoccupied_ids(&level));
    let stored = crate::entity::spawn_armor_stand(&mut store, position).unwrap();
    assert!(store.link_existing_passenger(boat, stored));
    attach_external_player_to_boat(&mut store, boat, player);
    assert_eq!(store.passengers(boat), &[player, stored]);
    assert!(store.set_external_player_boat_paddles(player, true, false));
    store.drain_lifecycle_events();

    for _ in 0..59 {
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    }
    assert_eq!(boat_data(&store, boat).out_of_control_ticks(), 59.0);
    assert_eq!(store.get(boat).unwrap().position(), position);
    assert_eq!(store.external_player_vehicle(player), Some(boat));
    // Only the paddle strokes (`AbstractBoat.java:251-271`) are heard.
    assert!(store
        .drain_lifecycle_events()
        .iter()
        .all(|event| matches!(event, EntityLifecycleEvent::Sound { .. })));

    level.with_level(|ctx| tick_entities(&mut store, ctx, true));

    assert_eq!(boat_data(&store, boat).out_of_control_ticks(), 60.0);
    assert_eq!(boat_data(&store, boat).paddle_state(), (false, false));
    assert_ne!(store.get(boat).unwrap().position(), position);
    assert_eq!(store.external_player_vehicle(player), None);
    assert_eq!(store.vehicle(stored), None);
    assert!(store.passengers(boat).is_empty());
}

#[test]
fn external_player_controlled_boat_timeout_and_bubble_eject_the_riders() {
    let position = Vec3d {
        x: 0.5,
        y: 69.5,
        z: 0.5,
    };
    let mut level = submerged_level_at(position);
    let player = joined_driver(&mut level);
    let (mut store, boat) = oak_boat_with_ids(position, unoccupied_ids(&level));
    let stored = crate::entity::spawn_armor_stand(&mut store, position).unwrap();
    assert!(store.link_existing_passenger(boat, stored));
    attach_external_player_to_boat(&mut store, boat, player);
    boat::set_out_of_control_ticks_for_test(store.get_mut(boat).unwrap(), 59.0);
    boat::set_bubble_countdown_for_test(store.get_mut(boat).unwrap(), 1, true);
    store.drain_lifecycle_events();

    level.with_level(|ctx| crate::entity::tick_one_entity(&mut store, ctx, boat));

    assert_eq!(boat_data(&store, boat).out_of_control_ticks(), 60.0);
    assert_eq!(boat_data(&store, boat).bubble_time(), 0);
    assert!(store.get(boat).unwrap().delta_movement().y < -0.7);
    assert_eq!(store.get(stored).unwrap().boarding_cooldown(), 60);
}

#[test]
fn external_player_controlled_boat_ejection_resets_player_and_gates_stored_reattach() {
    let position = Vec3d {
        x: 0.5,
        y: 69.5,
        z: 0.5,
    };
    let mut level = submerged_level_at(position);
    let player = joined_driver(&mut level);
    let (mut store, boat) = oak_boat_with_ids(position, unoccupied_ids(&level));
    let stored = crate::entity::spawn_armor_stand(&mut store, position).unwrap();
    assert!(store.link_existing_passenger(boat, stored));
    attach_external_player_to_boat(&mut store, boat, player);
    boat::set_out_of_control_ticks_for_test(store.get_mut(boat).unwrap(), 59.0);
    store.drain_lifecycle_events();
    level.with_level(|ctx| crate::entity::tick_one_entity(&mut store, ctx, boat));

    assert_eq!(store.get(stored).unwrap().boarding_cooldown(), 60);
    assert!(store.drain_lifecycle_events().iter().any(|event| matches!(
        event,
        EntityLifecycleEvent::ExternalPlayerBoardingCooldownReset {
            player_entity_id, ..
        } if *player_entity_id == player
    )));
    level.with_level(|ctx| tick_entities(&mut store, ctx, false));
    assert_eq!(store.get(stored).unwrap().boarding_cooldown(), 60);
    assert!(!store.link_existing_passenger(boat, stored));

    // `AbstractBoat.canAddPassenger` refuses while the boat's eyes are in
    // water, so the boat leaves the water before anyone boards again.
    let mut stored_level = NativeLevel::new();
    for x in -3..=3 {
        for z in -3..=3 {
            stored_level.world.set_block(
                DIMENSION,
                BlockPos { x, y: 69, z },
                state_id("minecraft:stone"),
                18,
            );
        }
    }
    stored_level.with_level(|ctx| crate::entity::tick_one_entity(&mut store, ctx, boat));
    assert!(store.attach_test_player(player, boat, crate::entity::ExternalPlayerCanRide::Allowed,));
    for _ in 0..59 {
        stored_level.with_level(|ctx| crate::entity::tick_one_entity(&mut store, ctx, stored));
    }
    assert_eq!(store.get(stored).unwrap().boarding_cooldown(), 1);
    assert!(!store.link_existing_passenger(boat, stored));
    stored_level.with_level(|ctx| crate::entity::tick_one_entity(&mut store, ctx, stored));
    assert_eq!(store.get(stored).unwrap().boarding_cooldown(), 0);
    assert!(store.link_existing_passenger(boat, stored));
    assert_eq!(store.external_player_vehicle(player), Some(boat));
}

#[test]
fn external_player_controlled_boat_upward_bubble_uses_player_launch() {
    let mut level = NativeLevel::new();
    let player = joined_driver(&mut level);
    let (mut store, boat) = oak_boat_with_ids(
        Vec3d {
            x: 0.5,
            y: 70.0,
            z: 0.5,
        },
        unoccupied_ids(&level),
    );
    attach_external_player_to_boat(&mut store, boat, player);
    store.drain_lifecycle_events();
    boat::on_above_bubble_column(store.get_mut(boat).unwrap(), false);

    for remaining in (0..BUBBLE_TIME).rev() {
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
        if remaining > 0 {
            boat::on_above_bubble_column(store.get_mut(boat).unwrap(), false);
        }
    }

    assert_eq!(boat_data(&store, boat).bubble_time(), 0);
    assert_eq!(store.get(boat).unwrap().delta_movement().y, 2.7);
    assert_eq!(store.external_player_vehicle(player), Some(boat));
    assert!(store.drain_lifecycle_events().is_empty());
}

#[test]
fn external_player_controlled_boat_downward_bubble_ejects_the_riders() {
    let mut level = NativeLevel::new();
    let position = Vec3d {
        x: 0.5,
        y: 70.0,
        z: 0.5,
    };
    let player = joined_driver(&mut level);
    let (mut store, boat) = oak_boat_with_ids(position, unoccupied_ids(&level));
    let stored = crate::entity::spawn_armor_stand(&mut store, position).unwrap();
    assert!(store.link_existing_passenger(boat, stored));
    attach_external_player_to_boat(&mut store, boat, player);
    store.drain_lifecycle_events();
    boat::on_above_bubble_column(store.get_mut(boat).unwrap(), true);

    for remaining in (0..BUBBLE_TIME).rev() {
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
        if remaining > 0 {
            boat::on_above_bubble_column(store.get_mut(boat).unwrap(), true);
        }
    }

    assert_eq!(store.get(boat).unwrap().delta_movement().y, -0.7);
    assert_eq!(store.external_player_vehicle(player), None);
    assert_eq!(store.vehicle(stored), None);
    assert!(store.passengers(boat).is_empty());
}

/// `:531-541`: a boat that was in the air and is now in water snaps to the
/// surface, drops its vertical delta and reports `IN_WATER` for the tick.
#[test]
fn a_boat_falling_into_water_snaps_to_the_surface() {
    let mut level = NativeLevel::new();
    for x in -3..=3 {
        for z in -3..=3 {
            level
                .world
                .set_block(DIMENSION, BlockPos { x, y: 69, z }, water_source(), 18);
        }
    }
    let (mut store, id) = oak_boat_at(Vec3d {
        x: 0.5,
        y: 70.0,
        z: 0.5,
    });
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    assert_eq!(boat_data(&store, id).status(), Some(BoatStatus::InAir));
    store.get_mut(id).expect("entity is live").set_pos(Vec3d {
        x: 0.5,
        y: 69.5,
        z: 0.5,
    });

    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    let data = boat_data(&store, id);
    assert_eq!(data.status(), Some(BoatStatus::InWater));
    // `getWaterLevelAbove` answers the first row that is not full water, which
    // is the air row above the surface.
    let entity = store.get(id).expect("entity is live");
    let expected_y = f64::from(70.0_f32 - 0.5625_f32) + 0.101;
    assert_eq!(entity.position().y, expected_y);
    assert_eq!(entity.delta_movement().y, 0.0);
    assert_eq!(data.last_yd(), 0.0);
}

/// `AbstractBoat.checkFallDamage` (`:707-716`) replaces the base accumulator:
/// it records `lastYd` and never adds fall distance over water.
#[test]
fn the_boat_fall_accumulator_records_last_yd_and_skips_water_below() {
    let mut level = NativeLevel::new();

    let (mut store, id) = oak_boat_at(Vec3d {
        x: 0.5,
        y: 70.0,
        z: 0.5,
    });
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    assert_eq!(boat_data(&store, id).last_yd(), -DEFAULT_GRAVITY);
    // `:712` narrows the tick's movement to a float before accumulating it.
    assert_eq!(
        store.get(id).expect("entity is live").fall_distance(),
        f64::from(DEFAULT_GRAVITY as f32)
    );

    let mut wet = NativeLevel::new();
    for x in -3..=3 {
        for z in -3..=3 {
            wet.world
                .set_block(DIMENSION, BlockPos { x, y: 68, z }, water_source(), 18);
        }
    }
    let (mut store, id) = oak_boat_at(Vec3d {
        x: 0.5,
        y: 70.0,
        z: 0.5,
    });
    wet.with_level(|ctx| tick_entities(&mut store, ctx, true));
    assert_eq!(store.get(id).expect("entity is live").fall_distance(), 0.0);
}

/// `AbstractBoat.onAboveBubbleColumn` (`:154-172`) is the only writer of the
/// flag `tickBubbleColumn` (`:308-329`) reads, and its upward column launches
/// the boat once the sixty ticks run out.
#[test]
fn a_bubble_column_launches_the_boat_when_its_timer_expires() {
    let mut level = NativeLevel::new();

    let (mut store, id) = oak_boat_at(Vec3d {
        x: 0.5,
        y: 70.0,
        z: 0.5,
    });
    boat::on_above_bubble_column(store.get_mut(id).expect("entity is live"), false);
    assert_eq!(boat_data(&store, id).bubble_time(), BUBBLE_TIME);

    // Every tick clears the flag again, so the timer runs down once.
    for remaining in (0..BUBBLE_TIME).rev() {
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
        assert_eq!(boat_data(&store, id).bubble_time(), remaining);
        if remaining > 0 {
            boat::on_above_bubble_column(store.get_mut(id).expect("entity is live"), false);
            assert_eq!(boat_data(&store, id).bubble_time(), remaining);
        }
    }
    // `:324`: the upward arm without a player passenger.
    assert_eq!(
        store.get(id).expect("entity is live").delta_movement().y,
        0.6
    );
}

#[test]
fn a_downward_bubble_column_adds_its_own_pull_instead() {
    let mut level = NativeLevel::new();

    let (mut store, id) = oak_boat_at(Vec3d {
        x: 0.5,
        y: 150.0,
        z: 0.5,
    });
    boat::on_above_bubble_column(store.get_mut(id).expect("entity is live"), true);
    for _ in 0..BUBBLE_TIME - 1 {
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
        boat::on_above_bubble_column(store.get_mut(id).expect("entity is live"), true);
    }
    let before = store.get(id).expect("entity is live").delta_movement().y;
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    let data = boat_data(&store, id);
    assert_eq!(data.bubble_time(), 0);
    // `:321-322`: the pull is added to whatever the float step left behind.
    let after = store.get(id).expect("entity is live").delta_movement().y;
    assert_eq!(after, before - DEFAULT_GRAVITY - 0.7);
}

#[test]
fn the_synched_rows_are_the_entity_vehicle_and_boat_rows_in_that_order() {
    let (store, id) = oak_boat_at(Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    });
    let entity = store.get(id).expect("entity is live");
    let data = boat::boat(entity.data()).expect("entity is a boat");
    let rows = boat::synched_values(entity, data);
    let base = crate::entity::living::entity_synched_values(entity);
    assert_eq!(rows.len(), base.len() + 6);
    let tail: Vec<(u8, i32, bool)> = rows[base.len()..]
        .iter()
        .map(|row| (row.value.data_id, row.value.serializer_id, row.is_default))
        .collect();
    assert_eq!(
        tail,
        vec![
            (
                VANILLA_VEHICLE_HURT_TIME_DATA_ID,
                VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                true
            ),
            (
                VANILLA_VEHICLE_HURT_DIR_DATA_ID,
                VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                true
            ),
            (
                VANILLA_VEHICLE_DAMAGE_DATA_ID,
                VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                true
            ),
            (
                VANILLA_ABSTRACT_BOAT_PADDLE_LEFT_DATA_ID,
                VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                true
            ),
            (
                VANILLA_ABSTRACT_BOAT_PADDLE_RIGHT_DATA_ID,
                VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                true
            ),
            (
                VANILLA_ABSTRACT_BOAT_BUBBLE_TIME_DATA_ID,
                VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                true
            ),
        ]
    );
}

// `VehicleEntity.hurtServer` (`VehicleEntity.java:35-59`)

#[test]
fn a_hit_flips_the_hurt_direction_marks_the_boat_and_banks_ten_times_the_damage() {
    let (mut level, id) = native_boat("minecraft:oak_boat");
    let outcome = level
        .world
        .hurt_stored_entity_by_command_for_test(DIMENSION, id, "minecraft:generic", 1.0, None)
        .unwrap();
    assert!(outcome.applied && outcome.succeeded);
    let entity = level.world.stored_entity_for_test(DIMENSION, id).unwrap();
    let data = boat::boat(entity.data()).unwrap();
    assert_eq!(
        (data.hurt_dir(), data.hurt_time(), data.damage()),
        (-1, 10, 10.0)
    );
    assert!(entity.hurt_marked());
    assert!(entity.is_alive());
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(|event| event.recorded.game_event
            == crate::level_event::VanillaGameEvent::ENTITY_DAMAGE
            && event.recorded.pos == pos(0, 70, 0)));
    level.world.tick(Vec::new());
    let entity = level.world.stored_entity_for_test(DIMENSION, id).unwrap();
    let data = boat::boat(entity.data()).unwrap();
    assert_eq!((data.hurt_time(), data.damage()), (9, 9.0));
}

#[test]
fn damage_past_the_threshold_breaks_the_boat_into_its_own_item() {
    for resource_id in BOAT_TYPES {
        let (mut level, id) = native_boat(resource_id);
        let outcome = level
            .world
            .hurt_stored_entity_by_command_for_test(
                DIMENSION,
                id,
                "minecraft:generic",
                DAMAGE_TO_DESTROY / 10.0 + 0.1,
                None,
            )
            .unwrap();
        assert!(outcome.applied && outcome.succeeded);
        assert!(level.world.stored_entity_for_test(DIMENSION, id).is_none());
        let items = level.world.stored_items_for_test(DIMENSION);
        assert!(items.iter().any(|item| item.2 == resource_id
            && item.3 == 1
            && item.1
                == Vec3d {
                    x: 0.5,
                    y: 70.0,
                    z: 0.5
                }));
    }
}

#[test]
fn entity_drops_false_removes_the_boat_without_spawning_its_item() {
    let (mut level, id) = native_boat("minecraft:oak_boat");
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut level.world,
        1,
        "gamerule minecraft:entity_drops false",
    );
    level
        .world
        .hurt_stored_entity_by_command_for_test(DIMENSION, id, "minecraft:generic", 5.0, None)
        .unwrap();
    assert!(level.world.stored_entity_for_test(DIMENSION, id).is_none());
    assert!(level.world.stored_items_for_test(DIMENSION).is_empty());
}

#[test]
fn only_a_mob_owned_explosion_with_mob_griefing_disabled_is_ignored() {
    assert!(boat::ignore_explosion(true, false));
    assert!(!boat::ignore_explosion(true, true));
    assert!(!boat::ignore_explosion(false, false));
    assert!(!boat::ignore_explosion(false, true));
    assert!(StoredDamageCausingEntity::Mob { entity_id: 1 }.is_mob());
    assert!(!StoredDamageCausingEntity::Other { entity_id: 2 }.is_mob());
    assert!(!StoredDamageCausingEntity::Player {
        uuid: Uuid::from_u128(3),
        entity_id: 3,
        instabuild: false,
    }
    .is_mob());
}

#[test]
fn a_creative_player_discards_the_boat_without_a_drop() {
    let (mut level, id) = native_boat("minecraft:oak_boat");
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut level.world, 1, "alice", 1);
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut level.world,
        1,
        "gamemode creative alice",
    );
    let uuid = level
        .world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .uuid();
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut level.world,
        2,
        &format!("damage {uuid} 100 minecraft:generic by alice"),
    );
    assert!(level.world.stored_entity_for_test(DIMENSION, id).is_none());
    assert!(level.world.stored_items_for_test(DIMENSION).is_empty());
}

#[test]
fn an_invulnerable_boat_refuses_a_hit_that_does_not_bypass_it() {
    let (mut level, id) = native_boat("minecraft:oak_boat");
    level
        .world
        .stored_entity_mut_for_test(DIMENSION, id)
        .unwrap()
        .set_invulnerable(true);
    let outcome = level
        .world
        .hurt_stored_entity_by_command_for_test(DIMENSION, id, "minecraft:generic", 1.0, None)
        .unwrap();
    assert!(!outcome.applied && !outcome.succeeded);
    assert_eq!(
        boat::boat(
            level
                .world
                .stored_entity_for_test(DIMENSION, id)
                .unwrap()
                .data()
        )
        .unwrap()
        .hurt_time(),
        0
    );
    assert!(!level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(|event| event.recorded.game_event
            == crate::level_event::VanillaGameEvent::ENTITY_DAMAGE));
}

#[test]
fn a_boat_round_trips_through_a_region_record_carrying_no_subtype_fields() {
    let position = Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    let (store, id) = oak_boat_at(position);
    let document = write_entity_chunk(&store, (0, 0)).expect("the boat writer exists");
    assert_eq!(document.entities().len(), 1);
    let record = document.entities().next().unwrap();
    assert_eq!(record.id(), "minecraft:oak_boat");
    // `AbstractBoat.addAdditionalSaveData` (`:675-678`) writes only the leash
    // payload, and `Leashable.writeLeashData` stores nothing for an absent one.
    assert!(record.raw().iter().all(|entry| entry.name != "leash"));

    let mut reloaded = EntityStore::new(0, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut reloaded, &document).expect("the boat reader exists");
    assert_eq!(ids.len(), 1);
    let restored = reloaded.get(ids[0]).expect("entity is live");
    assert_eq!(restored.uuid(), store.get(id).expect("live").uuid());
    assert_eq!(restored.position(), position);
    assert_eq!(
        boat::boat(restored.data()).expect("entity is a boat"),
        &BoatData::new()
    );
}

/// `AbstractBoat.readAdditionalSaveData` / `addAdditionalSaveData`
/// (`AbstractBoat.java:676-683`) are `Leashable`'s two codec calls and nothing
/// else.
#[test]
fn a_saved_boat_carries_its_leash_field_through_both_directions() {
    let leashed = vec![NbtSummaryEntry {
        name: "leash".into(),
        value: NbtSummaryValue::IntArray(vec![0, 64, 0]),
    }];
    let data = boat::read_save_data(&leashed, BoatData::new());
    let mut written = Vec::new();
    boat::write_save_data(&mut written, &data).expect("it re-encodes");
    assert_eq!(written, leashed);
}

#[test]
fn a_moving_boat_breaks_a_lily_pad_and_survives() {
    let (mut level, id) = native_boat("minecraft:oak_boat");
    let lily = pos(2, 70, 0);
    level.set(pos(2, 69, 0), water_source());
    level.set(lily, state_id("minecraft:lily_pad"));
    let before = level
        .world
        .with_level_for_test(DIMENSION, |ctx| ctx.block_state(lily));
    assert_eq!(before, state_id("minecraft:lily_pad"));
    level
        .world
        .stored_entity_mut_for_test(DIMENSION, id)
        .unwrap()
        .set_delta_movement(Vec3d {
            x: 2.0,
            y: 0.0,
            z: 0.0,
        });
    level.world.tick(Vec::new());
    let boat = level.world.stored_entity_for_test(DIMENSION, id).unwrap();
    assert!(boat.position().x > 0.5);
    assert!(boat.is_alive());
    assert_eq!(
        level.with_level(|ctx| ctx.block_state(lily)),
        state_id("minecraft:air")
    );
    let items = level.world.stored_items_for_test(DIMENSION);
    assert_eq!(
        items
            .iter()
            .filter(|item| item.2 == "minecraft:lily_pad")
            .map(|item| item.3)
            .sum::<i32>(),
        1
    );
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(|event| event.recorded.game_event
            == crate::level_event::VanillaGameEvent::BLOCK_DESTROY
            && event.recorded.source_entity_id == Some(id)));
    level
        .world
        .stored_entity_mut_for_test(DIMENSION, id)
        .unwrap()
        .set_delta_movement(Vec3d {
            x: 2.0,
            y: 0.0,
            z: 0.0,
        });
    level.world.tick(Vec::new());
    assert!(
        level
            .world
            .stored_entity_for_test(DIMENSION, id)
            .unwrap()
            .position()
            .x
            > 2.0
    );
}

#[test]
fn a_paddling_boat_on_land_plays_the_land_paddle_stroke_and_a_resting_one_is_silent() {
    let stone = state_id("minecraft:stone");
    let mut level = NativeLevel::new();
    for x in -3..=3 {
        for z in -3..=3 {
            level
                .world
                .set_block(DIMENSION, BlockPos { x, y: 69, z }, stone, 18);
        }
    }
    let land_paddle = bbb_protocol::packets::SoundEventHolder::Reference {
        registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
            "minecraft:entity.boat.paddle_land",
        )
        .unwrap(),
    };
    for paddling in [true, false] {
        let (mut store, id) = oak_boat_at(Vec3d {
            x: 0.5,
            y: 70.0,
            z: 0.5,
        });
        let mut entity = store.detach(id).unwrap();
        boat::set_paddle_state(&mut entity, paddling, false);
        store.drain_lifecycle_events();
        // `AbstractBoat.java:268`: one stroke is sixteen ticks.
        for _ in 0..16 {
            assert!(level
                .with_level(|ctx| boat::tick(&mut entity, &mut store, ctx, true, true))
                .is_some());
        }
        let strokes = store
            .drain_lifecycle_events()
            .iter()
            .filter(|event| {
                matches!(event, EntityLifecycleEvent::Sound { sound }
                    if sound.sound == land_paddle && sound.source == bbb_protocol::packets::SoundSource::Neutral)
            })
            .count();
        assert_eq!(strokes > 0, paddling);
    }
}

#[test]
fn a_boat_landing_on_cactus_breaks_under_its_own_tick() {
    let (mut level, id) = native_boat("minecraft:oak_boat");
    level.set(
        pos(0, 63, 0),
        crate::entity::tests::native_level::state("minecraft:sand"),
    );
    level.set(
        pos(0, 64, 0),
        crate::entity::tests::native_level::state("minecraft:cactus"),
    );
    for _ in 0..20 {
        level.world.tick(Vec::new());
        if level.world.stored_entity_for_test(DIMENSION, id).is_none() {
            break;
        }
    }
    assert!(level.world.stored_entity_for_test(DIMENSION, id).is_none());
    assert!(level
        .world
        .stored_items_for_test(DIMENSION)
        .iter()
        .any(|item| item.2 == "minecraft:oak_boat"));
}
