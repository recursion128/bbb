use anyhow::Result;
use bbb_protocol::packets::{self, PlayerPositionState};
use tokio::{sync::mpsc, time::Interval};

use crate::{
    connection::RawConnection,
    types::{ConnectionState, NetCommand},
};

mod commands;

pub(crate) use commands::{
    maybe_send_perform_respawn, send_attack_entity, send_chat_command,
    send_command_suggestion_request, send_container_button_click, send_container_click,
    send_container_close, send_container_slot_state_changed, send_interact_entity,
    send_pick_item_from_block, send_pick_item_from_entity, send_player_action, send_player_command,
    send_player_input_command, send_set_held_slot_command, send_swing_command, send_use_item,
    send_use_item_on,
};
use commands::{send_player_move_command, send_vehicle_move_command};

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
                    Some(NetCommand::ChatCommand(command)) => {
                        send_chat_command(conn, command).await?;
                    }
                    Some(NetCommand::AttackEntity(packet)) => {
                        send_attack_entity(conn, packet).await?;
                    }
                    Some(NetCommand::InteractEntity(packet)) => {
                        send_interact_entity(conn, packet).await?;
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
                    Some(NetCommand::PickItemFromEntity(packet)) => {
                        send_pick_item_from_entity(conn, packet).await?;
                    }
                    Some(NetCommand::ContainerButtonClick(packet)) => {
                        send_container_button_click(conn, packet).await?;
                    }
                    Some(NetCommand::ContainerClick(packet)) => {
                        send_container_click(conn, packet).await?;
                    }
                    Some(NetCommand::ContainerClose(packet)) => {
                        send_container_close(conn, packet).await?;
                    }
                    Some(NetCommand::ContainerSlotStateChanged(packet)) => {
                        send_container_slot_state_changed(conn, packet).await?;
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
                    Some(NetCommand::ChatCommand(_)) => {}
                    Some(NetCommand::AttackEntity(_)) => {}
                    Some(NetCommand::InteractEntity(_)) => {}
                    Some(NetCommand::SetHeldSlot(_)) => {}
                    Some(NetCommand::Swing(_)) => {}
                    Some(NetCommand::UseItemOn(_)) => {}
                    Some(NetCommand::UseItem(_)) => {}
                    Some(NetCommand::PickItemFromBlock(_)) => {}
                    Some(NetCommand::PickItemFromEntity(_)) => {}
                    Some(NetCommand::ContainerButtonClick(_)) => {}
                    Some(NetCommand::ContainerClick(_)) => {}
                    Some(NetCommand::ContainerClose(_)) => {}
                    Some(NetCommand::ContainerSlotStateChanged(_)) => {}
                    Some(NetCommand::CommandSuggestionRequest(_)) => {}
                    Some(NetCommand::Disconnect) | None => {
                        return Ok(ConnectionDrive::Disconnect);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
