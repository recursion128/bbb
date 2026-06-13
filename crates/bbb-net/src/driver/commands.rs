use anyhow::Result;
use bbb_protocol::packets::{
    self, CommandSuggestionRequest, InteractionHand, PickItemFromBlock, PlayerAction,
    PlayerCommand, PlayerHealth, PlayerInput, PlayerPositionState, UseItem, UseItemOn,
};

use crate::{
    connection::RawConnection,
    types::{PlayerMoveCommand, VehicleMoveCommand},
};

pub(super) async fn send_player_move_command(
    conn: &mut RawConnection,
    command: PlayerMoveCommand,
    player_position_state: &mut PlayerPositionState,
) -> Result<()> {
    let (id, payload) = command.encode_packet();
    conn.send_packet(id, &payload).await?;
    *player_position_state = command.state;
    Ok(())
}

pub(super) async fn send_vehicle_move_command(
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
    use super::{
        maybe_send_perform_respawn, send_command_suggestion_request, send_pick_item_from_block,
        send_player_action, send_player_command, send_player_input_command,
        send_set_held_slot_command, send_swing_command, send_use_item, send_use_item_on,
    };
    use crate::{
        connection::RawConnection,
        types::{PlayerMoveCommand, VehicleMoveCommand},
    };
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
    use tokio::time::timeout;

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
}
