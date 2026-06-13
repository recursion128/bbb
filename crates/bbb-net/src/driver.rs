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
