use anyhow::Result;
use bbb_protocol::packets::{
    self, CommandSuggestionRequest, InteractionHand, PickItemFromBlock, PlayerAction,
    PlayerCommand, PlayerHealth, PlayerInput, PlayerPositionState, UseItem, UseItemOn,
};
use tokio::{sync::mpsc, time::Interval};

use crate::{
    connection::RawConnection,
    types::{ConnectionState, NetCommand, PlayerMoveCommand, VehicleMoveCommand},
};

pub(crate) async fn read_packet_or_send_play_tick(
    conn: &mut RawConnection,
    state: ConnectionState,
    play_tick: &mut Option<Interval>,
) -> Result<(i32, Vec<u8>)> {
    if !matches!(state, ConnectionState::Play) {
        return conn.read_packet().await;
    }

    let Some(tick) = play_tick.as_mut() else {
        return conn.read_packet().await;
    };

    loop {
        tokio::select! {
            packet = conn.read_packet() => return packet,
            _ = tick.tick() => {
                let (id, payload) = packets::encode_play_client_tick_end();
                conn.send_packet(id, &payload).await?;
            }
        }
    }
}

#[derive(Debug)]
pub(crate) enum ConnectionDrive {
    Packet(i32, Vec<u8>),
    Disconnect,
}

pub(crate) async fn read_packet_or_drive_connection(
    conn: &mut RawConnection,
    state: ConnectionState,
    play_tick: &mut Option<Interval>,
    commands: &mut mpsc::Receiver<NetCommand>,
    player_position_state: &mut PlayerPositionState,
) -> Result<ConnectionDrive> {
    if !matches!(state, ConnectionState::Play) || play_tick.is_none() {
        return read_packet_or_disconnect_command(conn, commands).await;
    }
    let tick = play_tick.as_mut().expect("play tick checked above");

    loop {
        tokio::select! {
            packet = conn.read_packet() => {
                let (packet_id, payload) = packet?;
                return Ok(ConnectionDrive::Packet(packet_id, payload));
            }
            _ = tick.tick() => {
                let (id, payload) = packets::encode_play_client_tick_end();
                conn.send_packet(id, &payload).await?;
            }
            command = commands.recv() => {
                match command {
                    Some(NetCommand::MovePlayer(command)) => {
                        send_player_move_command(conn, command, player_position_state).await?;
                    }
                    Some(NetCommand::MoveVehicle(command)) => {
                        send_vehicle_move_command(conn, command).await?;
                    }
                    Some(NetCommand::PlayerAction(action)) => {
                        send_player_action(conn, action).await?;
                    }
                    Some(NetCommand::PlayerCommand(command)) => {
                        send_player_command(conn, command).await?;
                    }
                    Some(NetCommand::PlayerInput(input)) => {
                        send_player_input_command(conn, input).await?;
                    }
                    Some(NetCommand::SetHeldSlot(slot)) => {
                        send_set_held_slot_command(conn, slot).await?;
                    }
                    Some(NetCommand::Swing(hand)) => {
                        send_swing_command(conn, hand).await?;
                    }
                    Some(NetCommand::UseItemOn(packet)) => {
                        send_use_item_on(conn, packet).await?;
                    }
                    Some(NetCommand::UseItem(packet)) => {
                        send_use_item(conn, packet).await?;
                    }
                    Some(NetCommand::PickItemFromBlock(packet)) => {
                        send_pick_item_from_block(conn, packet).await?;
                    }
                    Some(NetCommand::CommandSuggestionRequest(request)) => {
                        send_command_suggestion_request(conn, request).await?;
                    }
                    Some(NetCommand::Disconnect) | None => {
                        return Ok(ConnectionDrive::Disconnect);
                    }
                }
            }
        }
    }
}

async fn read_packet_or_disconnect_command(
    conn: &mut RawConnection,
    commands: &mut mpsc::Receiver<NetCommand>,
) -> Result<ConnectionDrive> {
    loop {
        tokio::select! {
            packet = conn.read_packet() => {
                let (packet_id, payload) = packet?;
                return Ok(ConnectionDrive::Packet(packet_id, payload));
            }
            command = commands.recv() => {
                match command {
                    Some(NetCommand::MovePlayer(_)) => {}
                    Some(NetCommand::MoveVehicle(_)) => {}
                    Some(NetCommand::PlayerAction(_)) => {}
                    Some(NetCommand::PlayerCommand(_)) => {}
                    Some(NetCommand::PlayerInput(_)) => {}
                    Some(NetCommand::SetHeldSlot(_)) => {}
                    Some(NetCommand::Swing(_)) => {}
                    Some(NetCommand::UseItemOn(_)) => {}
                    Some(NetCommand::UseItem(_)) => {}
                    Some(NetCommand::PickItemFromBlock(_)) => {}
                    Some(NetCommand::CommandSuggestionRequest(_)) => {}
                    Some(NetCommand::Disconnect) | None => {
                        return Ok(ConnectionDrive::Disconnect);
                    }
                }
            }
        }
    }
}

async fn send_player_move_command(
    conn: &mut RawConnection,
    command: PlayerMoveCommand,
    player_position_state: &mut PlayerPositionState,
) -> Result<()> {
    let (id, payload) = command.encode_packet();
    conn.send_packet(id, &payload).await?;
    *player_position_state = command.state;
    Ok(())
}

async fn send_vehicle_move_command(
    conn: &mut RawConnection,
    command: VehicleMoveCommand,
) -> Result<()> {
    let (id, payload) = command.encode_packet();
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_player_action(
    conn: &mut RawConnection,
    action: PlayerAction,
) -> Result<()> {
    let (id, payload) = packets::encode_play_player_action(action);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_player_command(
    conn: &mut RawConnection,
    command: PlayerCommand,
) -> Result<()> {
    let (id, payload) = packets::encode_play_player_command(command);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_player_input_command(
    conn: &mut RawConnection,
    input: PlayerInput,
) -> Result<()> {
    let (id, payload) = packets::encode_play_player_input(input);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_set_held_slot_command(conn: &mut RawConnection, slot: u8) -> Result<()> {
    let (id, payload) = packets::encode_play_set_carried_item(i16::from(slot.min(8)));
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_swing_command(
    conn: &mut RawConnection,
    hand: InteractionHand,
) -> Result<()> {
    let (id, payload) = packets::encode_play_swing(hand);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_use_item_on(conn: &mut RawConnection, packet: UseItemOn) -> Result<()> {
    let (id, payload) = packets::encode_play_use_item_on(packet);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_use_item(conn: &mut RawConnection, packet: UseItem) -> Result<()> {
    let (id, payload) = packets::encode_play_use_item(packet);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_pick_item_from_block(
    conn: &mut RawConnection,
    packet: PickItemFromBlock,
) -> Result<()> {
    let (id, payload) = packets::encode_play_pick_item_from_block(packet);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_command_suggestion_request(
    conn: &mut RawConnection,
    request: CommandSuggestionRequest,
) -> Result<()> {
    let (id, payload) = packets::encode_play_command_suggestion_request(request);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn maybe_send_perform_respawn(
    conn: &mut RawConnection,
    health: PlayerHealth,
    player_was_dead: &mut bool,
) -> Result<()> {
    let is_dead = health.health <= 0.0;
    if is_dead && !*player_was_dead {
        let (id, payload) = packets::encode_play_perform_respawn();
        conn.send_packet(id, &payload).await?;
    }
    *player_was_dead = is_dead;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::{
        codec::Decoder,
        ids,
        packets::{
            CommandSuggestionRequest, InteractionHand, PlayerAction, PlayerCommand, PlayerHealth,
            PlayerInput, PlayerPositionState, Vec3d,
        },
    };
    use bytes::BytesMut;
    use std::time::Duration;
    use tokio::{sync::mpsc, time::timeout};

    #[tokio::test]
    async fn drive_connection_disconnects_when_command_channel_closes_before_play() {
        let (mut conn, server) = raw_connection_pair().await;
        let (tx, mut commands) = mpsc::channel(1);
        drop(tx);
        let mut play_tick = None;
        let mut player_position_state = PlayerPositionState::default();

        let result = timeout(
            Duration::from_secs(1),
            read_packet_or_drive_connection(
                &mut conn,
                ConnectionState::Login,
                &mut play_tick,
                &mut commands,
                &mut player_position_state,
            ),
        )
        .await
        .expect("drive should not hang")
        .unwrap();

        assert!(matches!(result, ConnectionDrive::Disconnect));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn drive_connection_honors_disconnect_command() {
        let (mut conn, server) = raw_connection_pair().await;
        let (tx, mut commands) = mpsc::channel(1);
        tx.send(NetCommand::Disconnect).await.unwrap();
        let mut play_tick = None;
        let mut player_position_state = PlayerPositionState::default();

        let result = timeout(
            Duration::from_secs(1),
            read_packet_or_drive_connection(
                &mut conn,
                ConnectionState::Configuration,
                &mut play_tick,
                &mut commands,
                &mut player_position_state,
            ),
        )
        .await
        .expect("drive should not hang")
        .unwrap();

        assert!(matches!(result, ConnectionDrive::Disconnect));
        server.await.unwrap();
    }

    #[test]
    fn player_move_command_encodes_pos_rot_packet() {
        let command = PlayerMoveCommand {
            state: PlayerPositionState {
                position: Vec3d {
                    x: 1.25,
                    y: 64.5,
                    z: -8.75,
                },
                delta_movement: Vec3d {
                    x: 0.1,
                    y: 0.0,
                    z: -0.2,
                },
                y_rot: 90.0,
                x_rot: -15.0,
            },
            on_ground: true,
            horizontal_collision: true,
        };

        let (packet_id, payload) = command.encode_packet();

        assert_eq!(packet_id, ids::play::SERVERBOUND_MOVE_PLAYER_POS_ROT);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_f64().unwrap(), 1.25);
        assert_eq!(decoder.read_f64().unwrap(), 64.5);
        assert_eq!(decoder.read_f64().unwrap(), -8.75);
        assert_eq!(decoder.read_f32().unwrap(), 90.0);
        assert_eq!(decoder.read_f32().unwrap(), -15.0);
        assert_eq!(decoder.read_u8().unwrap(), 0b11);
        assert!(decoder.is_empty());
    }

    #[test]
    fn vehicle_move_command_encodes_move_vehicle_packet() {
        let command = VehicleMoveCommand {
            position: Vec3d {
                x: 2.5,
                y: 70.0,
                z: -9.25,
            },
            y_rot: 180.0,
            x_rot: 12.5,
            on_ground: true,
        };

        let (packet_id, payload) = command.encode_packet();

        assert_eq!(packet_id, ids::play::SERVERBOUND_MOVE_VEHICLE);
        assert_eq!(payload.len(), 33);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_f64().unwrap(), 2.5);
        assert_eq!(decoder.read_f64().unwrap(), 70.0);
        assert_eq!(decoder.read_f64().unwrap(), -9.25);
        assert_eq!(decoder.read_f32().unwrap(), 180.0);
        assert_eq!(decoder.read_f32().unwrap(), 12.5);
        assert_eq!(decoder.read_bool().unwrap(), true);
        assert!(decoder.is_empty());
    }

    #[tokio::test]
    async fn send_player_action_encodes_player_action_packet() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = RawConnection {
                stream,
                read_buf: BytesMut::new(),
                compression_threshold: None,
            };
            let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
                .await
                .expect("player action should be sent")
                .unwrap();
            assert_eq!(packet_id, ids::play::SERVERBOUND_PLAYER_ACTION);
            let mut decoder = Decoder::new(&payload);
            assert_eq!(decoder.read_var_i32().unwrap(), 0);
            let pos = decoder.read_i64().unwrap();
            assert_eq!(
                bbb_protocol::packets::BlockPos {
                    x: (pos >> 38) as i32,
                    y: ((pos << 52) >> 52) as i32,
                    z: ((pos << 26) >> 38) as i32,
                },
                bbb_protocol::packets::BlockPos { x: 1, y: 64, z: -2 }
            );
            assert_eq!(decoder.read_u8().unwrap(), 4);
            assert_eq!(decoder.read_var_i32().unwrap(), 9);
            assert!(decoder.is_empty());
        });
        let mut conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();

        send_player_action(
            &mut conn,
            PlayerAction {
                action: bbb_protocol::packets::PlayerActionKind::StartDestroyBlock,
                pos: bbb_protocol::packets::BlockPos { x: 1, y: 64, z: -2 },
                direction: bbb_protocol::packets::Direction::West,
                sequence: 9,
            },
        )
        .await
        .unwrap();

        server.await.unwrap();
    }

    #[tokio::test]
    async fn send_player_command_encodes_player_command_packet() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = RawConnection {
                stream,
                read_buf: BytesMut::new(),
                compression_threshold: None,
            };
            let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
                .await
                .expect("player command should be sent")
                .unwrap();
            assert_eq!(packet_id, ids::play::SERVERBOUND_PLAYER_COMMAND);
            let mut decoder = Decoder::new(&payload);
            assert_eq!(decoder.read_var_i32().unwrap(), 123);
            assert_eq!(decoder.read_var_i32().unwrap(), 2);
            assert_eq!(decoder.read_var_i32().unwrap(), 0);
            assert!(decoder.is_empty());
        });
        let mut conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();

        send_player_command(
            &mut conn,
            PlayerCommand {
                entity_id: 123,
                action: bbb_protocol::packets::PlayerCommandAction::StopSprinting,
                data: 0,
            },
        )
        .await
        .unwrap();

        server.await.unwrap();
    }

    #[tokio::test]
    async fn send_player_input_command_encodes_input_packet() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = RawConnection {
                stream,
                read_buf: BytesMut::new(),
                compression_threshold: None,
            };
            let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
                .await
                .expect("player input command should be sent")
                .unwrap();
            assert_eq!(packet_id, ids::play::SERVERBOUND_PLAYER_INPUT);
            let mut decoder = Decoder::new(&payload);
            assert_eq!(decoder.read_u8().unwrap(), 0b0111_0001);
            assert!(decoder.is_empty());
        });
        let mut conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();

        send_player_input_command(
            &mut conn,
            PlayerInput {
                forward: true,
                backward: false,
                left: false,
                right: false,
                jump: true,
                shift: true,
                sprint: true,
            },
        )
        .await
        .unwrap();

        server.await.unwrap();
    }

    #[tokio::test]
    async fn send_set_held_slot_command_encodes_carried_item_packet() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = RawConnection {
                stream,
                read_buf: BytesMut::new(),
                compression_threshold: None,
            };
            let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
                .await
                .expect("held-slot command should be sent")
                .unwrap();
            assert_eq!(packet_id, ids::play::SERVERBOUND_SET_CARRIED_ITEM);
            let mut decoder = Decoder::new(&payload);
            assert_eq!(decoder.read_i16().unwrap(), 8);
            assert!(decoder.is_empty());
        });
        let mut conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();

        send_set_held_slot_command(&mut conn, 12).await.unwrap();

        server.await.unwrap();
    }

    #[tokio::test]
    async fn send_swing_command_encodes_swing_packet() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = RawConnection {
                stream,
                read_buf: BytesMut::new(),
                compression_threshold: None,
            };
            let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
                .await
                .expect("swing command should be sent")
                .unwrap();
            assert_eq!(packet_id, ids::play::SERVERBOUND_SWING);
            let mut decoder = Decoder::new(&payload);
            assert_eq!(decoder.read_var_i32().unwrap(), 0);
            assert!(decoder.is_empty());
        });
        let mut conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();

        send_swing_command(&mut conn, InteractionHand::MainHand)
            .await
            .unwrap();

        server.await.unwrap();
    }

    #[tokio::test]
    async fn send_use_item_on_encodes_use_item_on_packet() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = RawConnection {
                stream,
                read_buf: BytesMut::new(),
                compression_threshold: None,
            };
            let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
                .await
                .expect("use item on command should be sent")
                .unwrap();
            assert_eq!(packet_id, ids::play::SERVERBOUND_USE_ITEM_ON);
            let mut decoder = Decoder::new(&payload);
            assert_eq!(decoder.read_var_i32().unwrap(), 0);
            let pos = decoder.read_i64().unwrap();
            assert_eq!(
                bbb_protocol::packets::BlockPos {
                    x: (pos >> 38) as i32,
                    y: ((pos << 52) >> 52) as i32,
                    z: ((pos << 26) >> 38) as i32,
                },
                bbb_protocol::packets::BlockPos {
                    x: -5,
                    y: 70,
                    z: 12
                }
            );
            assert_eq!(decoder.read_var_i32().unwrap(), 3);
            assert_eq!(decoder.read_f32().unwrap(), 0.25);
            assert_eq!(decoder.read_f32().unwrap(), 0.5);
            assert_eq!(decoder.read_f32().unwrap(), 0.75);
            assert!(!decoder.read_bool().unwrap());
            assert!(!decoder.read_bool().unwrap());
            assert_eq!(decoder.read_var_i32().unwrap(), 4);
            assert!(decoder.is_empty());
        });
        let mut conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();

        send_use_item_on(
            &mut conn,
            bbb_protocol::packets::UseItemOn {
                hand: InteractionHand::MainHand,
                hit: bbb_protocol::packets::BlockHitResult {
                    pos: bbb_protocol::packets::BlockPos {
                        x: -5,
                        y: 70,
                        z: 12,
                    },
                    direction: bbb_protocol::packets::Direction::South,
                    cursor_x: 0.25,
                    cursor_y: 0.5,
                    cursor_z: 0.75,
                    inside: false,
                    world_border_hit: false,
                },
                sequence: 4,
            },
        )
        .await
        .unwrap();

        server.await.unwrap();
    }

    #[tokio::test]
    async fn send_use_item_encodes_use_item_packet() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = RawConnection {
                stream,
                read_buf: BytesMut::new(),
                compression_threshold: None,
            };
            let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
                .await
                .expect("use item command should be sent")
                .unwrap();
            assert_eq!(packet_id, ids::play::SERVERBOUND_USE_ITEM);
            let mut decoder = Decoder::new(&payload);
            assert_eq!(decoder.read_var_i32().unwrap(), 1);
            assert_eq!(decoder.read_var_i32().unwrap(), 8);
            assert_eq!(decoder.read_f32().unwrap(), 45.0);
            assert_eq!(decoder.read_f32().unwrap(), -20.0);
            assert!(decoder.is_empty());
        });
        let mut conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();

        send_use_item(
            &mut conn,
            bbb_protocol::packets::UseItem {
                hand: InteractionHand::OffHand,
                sequence: 8,
                y_rot: 45.0,
                x_rot: -20.0,
            },
        )
        .await
        .unwrap();

        server.await.unwrap();
    }

    #[tokio::test]
    async fn send_pick_item_from_block_encodes_pick_packet() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = RawConnection {
                stream,
                read_buf: BytesMut::new(),
                compression_threshold: None,
            };
            let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
                .await
                .expect("pick item from block command should be sent")
                .unwrap();
            assert_eq!(packet_id, ids::play::SERVERBOUND_PICK_ITEM_FROM_BLOCK);
            let mut decoder = Decoder::new(&payload);
            let pos = decoder.read_i64().unwrap();
            assert_eq!(
                bbb_protocol::packets::BlockPos {
                    x: (pos >> 38) as i32,
                    y: ((pos << 52) >> 52) as i32,
                    z: ((pos << 26) >> 38) as i32,
                },
                bbb_protocol::packets::BlockPos {
                    x: -5,
                    y: 70,
                    z: 12
                }
            );
            assert!(decoder.read_bool().unwrap());
            assert!(decoder.is_empty());
        });
        let mut conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();

        send_pick_item_from_block(
            &mut conn,
            bbb_protocol::packets::PickItemFromBlock {
                pos: bbb_protocol::packets::BlockPos {
                    x: -5,
                    y: 70,
                    z: 12,
                },
                include_data: true,
            },
        )
        .await
        .unwrap();

        server.await.unwrap();
    }

    #[tokio::test]
    async fn send_command_suggestion_request_encodes_command_suggestion_packet() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = RawConnection {
                stream,
                read_buf: BytesMut::new(),
                compression_threshold: None,
            };
            let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
                .await
                .expect("command suggestion request should be sent")
                .unwrap();
            assert_eq!(packet_id, ids::play::SERVERBOUND_COMMAND_SUGGESTION);
            let mut decoder = Decoder::new(&payload);
            assert_eq!(decoder.read_var_i32().unwrap(), 44);
            assert_eq!(
                decoder.read_string(32500).unwrap(),
                "/give @p minecraft:stone"
            );
            assert!(decoder.is_empty());
        });
        let mut conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();

        send_command_suggestion_request(
            &mut conn,
            CommandSuggestionRequest {
                id: 44,
                command: "/give @p minecraft:stone".to_string(),
            },
        )
        .await
        .unwrap();

        server.await.unwrap();
    }

    #[tokio::test]
    async fn death_health_sends_respawn_command_once_until_alive() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = RawConnection {
                stream,
                read_buf: BytesMut::new(),
                compression_threshold: None,
            };
            let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
                .await
                .expect("respawn command should be sent")
                .unwrap();
            assert_eq!(packet_id, ids::play::SERVERBOUND_CLIENT_COMMAND);
            let mut decoder = Decoder::new(&payload);
            assert_eq!(decoder.read_var_i32().unwrap(), 0);
            assert!(decoder.is_empty());

            assert!(
                timeout(Duration::from_millis(100), conn.read_packet())
                    .await
                    .is_err(),
                "second dead health packet must not send another respawn"
            );
        });
        let mut conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();
        let mut player_was_dead = false;

        maybe_send_perform_respawn(
            &mut conn,
            PlayerHealth {
                health: 20.0,
                food: 20,
                saturation: 5.0,
            },
            &mut player_was_dead,
        )
        .await
        .unwrap();
        assert!(!player_was_dead);

        maybe_send_perform_respawn(
            &mut conn,
            PlayerHealth {
                health: 0.0,
                food: 18,
                saturation: 0.0,
            },
            &mut player_was_dead,
        )
        .await
        .unwrap();
        assert!(player_was_dead);

        maybe_send_perform_respawn(
            &mut conn,
            PlayerHealth {
                health: 0.0,
                food: 18,
                saturation: 0.0,
            },
            &mut player_was_dead,
        )
        .await
        .unwrap();

        server.await.unwrap();
    }

    async fn raw_connection_pair() -> (RawConnection, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (_stream, _) = listener.accept().await.unwrap();
            tokio::time::sleep(Duration::from_millis(50)).await;
        });
        let conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();
        (conn, server)
    }
}
