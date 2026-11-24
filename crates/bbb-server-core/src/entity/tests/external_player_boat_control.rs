use super::*;
use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};
use crate::mailbox::{WorldCommandReceiver, WorldCommandSender};
use crate::world::{TickResult, WorldCommand, WorldEvent};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID;
use bbb_protocol::packets::{InteractEntity, InteractionHand, MoveVehicleCommand, PlayServerbound};

fn packet(
    level: &mut NativeLevel,
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    packet: PlayServerbound,
) -> TickResult {
    sender
        .try_send(WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet,
        })
        .unwrap();
    level.world.tick(receiver.drain())
}

fn mounted_boat() -> (NativeLevel, WorldCommandSender, WorldCommandReceiver, i32) {
    let mut level = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut level.world, 1, "alice", 1);
    let player = level.world.publication().players[0].position;
    let (boat, _) = level.world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        Vec3d {
            y: player.y + 2.0,
            ..player
        },
        EntityData::Boat(crate::entity::BoatData::new()),
    );
    let mounted = packet(
        &mut level,
        &sender,
        &receiver,
        PlayServerbound::Interact(InteractEntity {
            entity_id: boat,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    );
    let teleport = mounted
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(position),
                ..
            } => Some(position.id),
            _ => None,
        })
        .unwrap();
    packet(
        &mut level,
        &sender,
        &receiver,
        PlayServerbound::AcceptTeleportation { id: teleport },
    );
    assert_eq!(
        level
            .world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .2,
        Some(boat)
    );
    (level, sender, receiver, boat)
}

#[test]
fn external_player_boat_control_wraps_rotation_and_refiles_a_live_move() {
    let (mut level, sender, receiver, boat) = mounted_boat();
    let start = level
        .world
        .stored_entity_for_test(DIMENSION, boat)
        .unwrap()
        .position();
    let section = level
        .world
        .stored_entity_for_test(DIMENSION, boat)
        .unwrap()
        .filed_section();
    for step in 1..=34 {
        let target = Vec3d {
            x: start.x + f64::from(step) * 0.5,
            ..start
        };
        let moved = packet(
            &mut level,
            &sender,
            &receiver,
            PlayServerbound::MoveVehicle(MoveVehicleCommand {
                position: target,
                y_rot: 540.0,
                x_rot: -540.0,
                on_ground: false,
            }),
        );
        assert!(!moved.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::MoveVehicle(_),
                ..
            }
        )));
        let entity = level.world.stored_entity_for_test(DIMENSION, boat).unwrap();
        assert_eq!(entity.position(), target);
        assert_eq!((entity.y_rot(), entity.x_rot()), (-180.0, -180.0));
    }
    assert_ne!(
        level
            .world
            .stored_entity_for_test(DIMENSION, boat)
            .unwrap()
            .filed_section(),
        section
    );
}

#[test]
fn external_player_boat_control_can_leave_existing_block_collision() {
    let (mut level, sender, receiver, boat) = mounted_boat();
    let start = level
        .world
        .stored_entity_for_test(DIMENSION, boat)
        .unwrap()
        .position();
    let cell = level
        .world
        .stored_entity_for_test(DIMENSION, boat)
        .unwrap()
        .block_position();
    level.set(cell, state_id("minecraft:stone"));
    let target = Vec3d {
        x: start.x + 0.25,
        ..start
    };
    let moved = packet(
        &mut level,
        &sender,
        &receiver,
        PlayServerbound::MoveVehicle(MoveVehicleCommand {
            position: target,
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: false,
        }),
    );
    assert!(!moved.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::MoveVehicle(_),
            ..
        }
    )));
    let entity = level.world.stored_entity_for_test(DIMENSION, boat).unwrap();
    assert_eq!(entity.position(), target);
}

#[test]
fn external_player_boat_control_landing_updates_support_and_resets_fall_distance() {
    let (mut level, sender, receiver, boat) = mounted_boat();
    let start = level
        .world
        .stored_entity_for_test(DIMENSION, boat)
        .unwrap()
        .position();
    level
        .world
        .stored_entity_mut_for_test(DIMENSION, boat)
        .unwrap()
        .fall_distance = 2.0;
    let target = Vec3d { y: 64.0, ..start };
    packet(
        &mut level,
        &sender,
        &receiver,
        PlayServerbound::MoveVehicle(MoveVehicleCommand {
            position: target,
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: true,
        }),
    );
    let entity = level.world.stored_entity_for_test(DIMENSION, boat).unwrap();
    assert_eq!(entity.position(), target);
    assert!(entity.on_ground());
    assert!(entity.is_supported_by(BlockPos {
        x: target.x.floor() as i32,
        y: 63,
        z: target.z.floor() as i32
    }));
    assert_eq!(entity.fall_distance(), 0.0);
}

#[test]
fn external_player_boat_control_rejects_a_nonfinite_rotation() {
    let (mut level, sender, receiver, boat) = mounted_boat();
    let start = level
        .world
        .stored_entity_for_test(DIMENSION, boat)
        .unwrap()
        .position();
    let invalid = packet(
        &mut level,
        &sender,
        &receiver,
        PlayServerbound::MoveVehicle(MoveVehicleCommand {
            position: start,
            y_rot: f32::INFINITY,
            x_rot: 0.0,
            on_ground: false,
        }),
    );
    assert!(invalid.events.iter().any(|event| matches!(
        event,
        WorldEvent::Close {
            connection_id: ConnectionId(1),
            reason: crate::world::WorldCloseReason::InvalidVehicleMovement
        }
    )));
}

#[test]
fn external_player_boat_control_refuses_a_player_with_eyes_in_actual_water() {
    let mut level = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut level.world, 1, "alice", 1);
    let player = level.world.publication().players[0].position;
    let cell = bbb_protocol::packets::BlockPos {
        x: player.x.floor() as i32,
        y: player.y.floor() as i32,
        z: player.z.floor() as i32,
    };
    for y in cell.y..=cell.y + 1 {
        level.world.set_block(
            DIMENSION,
            BlockPos { y, ..cell },
            state_id("minecraft:water"),
            18,
        );
    }
    level.world.tick(Vec::new());
    let (boat, _) = level.world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        Vec3d {
            y: player.y + 4.0,
            ..player
        },
        EntityData::Boat(crate::entity::BoatData::new()),
    );
    packet(
        &mut level,
        &sender,
        &receiver,
        PlayServerbound::Interact(InteractEntity {
            entity_id: boat,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    );
    assert_eq!(
        level
            .world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .2,
        None
    );
}

#[test]
fn external_player_boat_control_only_the_joined_driver_changes_paddles() {
    let (mut level, sender, receiver, boat) = mounted_boat();
    crate::tests::join_loaded(&sender, &receiver, &mut level.world, 2, "bob", 2);
    sender
        .try_send(WorldCommand::Play {
            connection_id: ConnectionId(2),
            packet: PlayServerbound::Interact(InteractEntity {
                entity_id: boat,
                hand: InteractionHand::MainHand,
                location: Vec3d::default(),
                using_secondary_action: false,
            }),
        })
        .unwrap();
    level.world.tick(receiver.drain());
    assert_eq!(
        level
            .world
            .external_player_vehicle_state_for_test(ConnectionId(2))
            .2,
        Some(boat)
    );
    packet(
        &mut level,
        &sender,
        &receiver,
        PlayServerbound::PaddleBoat(bbb_protocol::packets::PaddleBoat {
            left: true,
            right: false,
        }),
    );
    sender
        .try_send(WorldCommand::Play {
            connection_id: ConnectionId(2),
            packet: PlayServerbound::PaddleBoat(bbb_protocol::packets::PaddleBoat {
                left: true,
                right: true,
            }),
        })
        .unwrap();
    level.world.tick(receiver.drain());
    assert_eq!(
        level.world.boat_control_state_for_test(DIMENSION, boat).3,
        (true, false)
    );
}
