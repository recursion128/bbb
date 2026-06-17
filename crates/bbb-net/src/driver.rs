use anyhow::Result;
use bbb_protocol::packets::{self, PlayerPositionState};
use tokio::{sync::mpsc, time::Interval};

use crate::{
    connection::RawConnection,
    types::{ConnectionState, NetCommand},
};

mod commands;

pub(crate) use commands::{
    maybe_send_perform_respawn, send_accept_code_of_conduct, send_attack_entity, send_chat_command,
    send_command_suggestion_request, send_container_button_click, send_container_click,
    send_container_close, send_container_slot_state_changed, send_interact_entity,
    send_paddle_boat, send_pick_item_from_block, send_pick_item_from_entity, send_ping_request,
    send_place_recipe, send_player_abilities_command, send_player_action, send_player_command,
    send_player_input_command, send_recipe_book_change_settings, send_recipe_book_seen_recipe,
    send_select_bundle_item, send_select_trade, send_set_held_slot_command, send_sign_update,
    send_swing_command, send_use_item, send_use_item_on,
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
        return read_packet_or_disconnect_command(conn, state, commands).await;
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
                    Some(NetCommand::PlayerAbilities(command)) => {
                        send_player_abilities_command(conn, command).await?;
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
                    Some(NetCommand::PaddleBoat(packet)) => {
                        send_paddle_boat(conn, packet).await?;
                    }
                    Some(NetCommand::PingRequest(time)) => {
                        send_ping_request(conn, time).await?;
                    }
                    Some(NetCommand::PlaceRecipe(command)) => {
                        send_place_recipe(conn, command).await?;
                    }
                    Some(NetCommand::RecipeBookChangeSettings(command)) => {
                        send_recipe_book_change_settings(conn, command).await?;
                    }
                    Some(NetCommand::RecipeBookSeenRecipe(command)) => {
                        send_recipe_book_seen_recipe(conn, command).await?;
                    }
                    Some(NetCommand::SelectTrade(command)) => {
                        send_select_trade(conn, command).await?;
                    }
                    Some(NetCommand::SignUpdate(packet)) => {
                        send_sign_update(conn, packet).await?;
                    }
                    Some(NetCommand::SelectBundleItem(packet)) => {
                        send_select_bundle_item(conn, packet).await?;
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
                    Some(NetCommand::AcceptCodeOfConduct) => {}
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
    state: ConnectionState,
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
                    Some(NetCommand::PlayerAbilities(_)) => {}
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
                    Some(NetCommand::PaddleBoat(_)) => {}
                    Some(NetCommand::PingRequest(_)) => {}
                    Some(NetCommand::PlaceRecipe(_)) => {}
                    Some(NetCommand::RecipeBookChangeSettings(_)) => {}
                    Some(NetCommand::RecipeBookSeenRecipe(_)) => {}
                    Some(NetCommand::SelectTrade(_)) => {}
                    Some(NetCommand::SignUpdate(_)) => {}
                    Some(NetCommand::SelectBundleItem(_)) => {}
                    Some(NetCommand::ContainerButtonClick(_)) => {}
                    Some(NetCommand::ContainerClick(_)) => {}
                    Some(NetCommand::ContainerClose(_)) => {}
                    Some(NetCommand::ContainerSlotStateChanged(_)) => {}
                    Some(NetCommand::CommandSuggestionRequest(_)) => {}
                    Some(NetCommand::AcceptCodeOfConduct) => {
                        if matches!(state, ConnectionState::Configuration) {
                            send_accept_code_of_conduct(conn).await?;
                        }
                    }
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
    use bbb_protocol::{
        codec::Decoder,
        ids,
        packets::{
            AttackEntity, BlockHitResult, BlockPos, ChatCommand, CommandSuggestionRequest,
            ContainerButtonClick, ContainerClick, ContainerCloseRequest, ContainerInput,
            ContainerSlotStateChanged, Direction, HashedStack, InteractEntity, InteractionHand,
            PaddleBoat, PickItemFromBlock, PickItemFromEntity, PlaceRecipeCommand,
            PlayerAbilitiesCommand, PlayerAction, PlayerActionKind,
            RecipeBookChangeSettingsCommand, RecipeBookSeenRecipeCommand, RecipeBookType,
            RecipeDisplayId, SelectBundleItem, SelectTradeCommand, SignUpdate, UseItem, UseItemOn,
            Vec3d,
        },
    };
    use bytes::BytesMut;
    use std::{collections::BTreeMap, time::Duration};
    use tokio::{
        sync::mpsc,
        time::{interval_at, timeout, Instant as TokioInstant},
    };

    use crate::types::{PlayerMoveCommand, VehicleMoveCommand};

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

    #[tokio::test]
    async fn drive_connection_sends_code_of_conduct_accept_in_configuration() {
        let (mut conn, server) = raw_connection_pair_expect_accept_code_of_conduct().await;
        let (tx, mut commands) = mpsc::channel(2);
        tx.send(NetCommand::AcceptCodeOfConduct).await.unwrap();
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

    #[tokio::test]
    async fn drive_connection_sends_movement_net_commands_in_play() {
        let (mut conn, mut server) = raw_connection_pair_with_server().await;
        let (tx, mut commands) = mpsc::channel(3);
        let move_state = PlayerPositionState {
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
        };
        tx.send(NetCommand::MovePlayer(PlayerMoveCommand {
            state: move_state,
            on_ground: true,
            horizontal_collision: false,
            force_position: false,
        }))
        .await
        .unwrap();
        tx.send(NetCommand::MoveVehicle(VehicleMoveCommand {
            position: Vec3d {
                x: 2.5,
                y: 70.0,
                z: -9.25,
            },
            y_rot: 180.0,
            x_rot: 12.5,
            on_ground: true,
        }))
        .await
        .unwrap();
        tx.send(NetCommand::Disconnect).await.unwrap();
        let mut player_position_state = PlayerPositionState::default();

        drive_play_until_disconnect(&mut conn, &mut commands, &mut player_position_state).await;

        assert_eq!(player_position_state, move_state);
        let (packet_id, payload) = read_server_packet(&mut server, "move player").await;
        assert_eq!(packet_id, ids::play::SERVERBOUND_MOVE_PLAYER_POS_ROT);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_f64().unwrap(), 1.25);
        assert_eq!(decoder.read_f64().unwrap(), 64.5);
        assert_eq!(decoder.read_f64().unwrap(), -8.75);
        assert_eq!(decoder.read_f32().unwrap(), 90.0);
        assert_eq!(decoder.read_f32().unwrap(), -15.0);
        assert_eq!(decoder.read_u8().unwrap(), 0b01);
        assert!(decoder.is_empty());

        let (packet_id, payload) = read_server_packet(&mut server, "move vehicle").await;
        assert_eq!(packet_id, ids::play::SERVERBOUND_MOVE_VEHICLE);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_f64().unwrap(), 2.5);
        assert_eq!(decoder.read_f64().unwrap(), 70.0);
        assert_eq!(decoder.read_f64().unwrap(), -9.25);
        assert_eq!(decoder.read_f32().unwrap(), 180.0);
        assert_eq!(decoder.read_f32().unwrap(), 12.5);
        assert!(decoder.read_bool().unwrap());
        assert!(decoder.is_empty());
    }

    #[tokio::test]
    async fn drive_connection_sends_chat_and_command_net_commands_in_play() {
        let (mut conn, mut server) = raw_connection_pair_with_server().await;
        let (tx, mut commands) = mpsc::channel(3);
        tx.send(NetCommand::ChatCommand(ChatCommand {
            command: "give @p minecraft:stone".to_string(),
        }))
        .await
        .unwrap();
        tx.send(NetCommand::CommandSuggestionRequest(
            CommandSuggestionRequest {
                id: 44,
                command: "/give @p minecraft:stone".to_string(),
            },
        ))
        .await
        .unwrap();
        tx.send(NetCommand::Disconnect).await.unwrap();
        let mut player_position_state = PlayerPositionState::default();

        drive_play_until_disconnect(&mut conn, &mut commands, &mut player_position_state).await;

        let (packet_id, payload) = read_server_packet(&mut server, "chat command").await;
        assert_eq!(packet_id, ids::play::SERVERBOUND_CHAT_COMMAND);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(
            decoder.read_string(32767).unwrap(),
            "give @p minecraft:stone"
        );
        assert!(decoder.is_empty());

        let (packet_id, payload) =
            read_server_packet(&mut server, "command suggestion request").await;
        assert_eq!(packet_id, ids::play::SERVERBOUND_COMMAND_SUGGESTION);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 44);
        assert_eq!(
            decoder.read_string(32767).unwrap(),
            "/give @p minecraft:stone"
        );
        assert!(decoder.is_empty());
    }

    #[tokio::test]
    async fn drive_connection_sends_player_abilities_net_command_in_play() {
        let (mut conn, mut server) = raw_connection_pair_with_server().await;
        let (tx, mut commands) = mpsc::channel(2);
        tx.send(NetCommand::PlayerAbilities(PlayerAbilitiesCommand {
            flying: true,
        }))
        .await
        .unwrap();
        tx.send(NetCommand::Disconnect).await.unwrap();
        let mut player_position_state = PlayerPositionState::default();

        drive_play_until_disconnect(&mut conn, &mut commands, &mut player_position_state).await;

        let (packet_id, payload) = read_server_packet(&mut server, "player abilities").await;
        assert_eq!(packet_id, ids::play::SERVERBOUND_PLAYER_ABILITIES);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_u8().unwrap(), 0x02);
        assert!(decoder.is_empty());
    }

    #[tokio::test]
    async fn drive_connection_sends_place_recipe_net_command_in_play() {
        let (mut conn, mut server) = raw_connection_pair_with_server().await;
        let (tx, mut commands) = mpsc::channel(2);
        tx.send(NetCommand::PlaceRecipe(PlaceRecipeCommand {
            container_id: 7,
            recipe_index: 123,
            use_max_items: true,
        }))
        .await
        .unwrap();
        tx.send(NetCommand::Disconnect).await.unwrap();
        let mut player_position_state = PlayerPositionState::default();

        drive_play_until_disconnect(&mut conn, &mut commands, &mut player_position_state).await;

        let (packet_id, payload) = read_server_packet(&mut server, "place recipe").await;
        assert_eq!(packet_id, ids::play::SERVERBOUND_PLACE_RECIPE);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 7);
        assert_eq!(decoder.read_var_i32().unwrap(), 123);
        assert!(decoder.read_bool().unwrap());
        assert!(decoder.is_empty());
    }

    #[tokio::test]
    async fn drive_connection_sends_recipe_book_net_commands_in_play() {
        let (mut conn, mut server) = raw_connection_pair_with_server().await;
        let (tx, mut commands) = mpsc::channel(3);
        tx.send(NetCommand::RecipeBookChangeSettings(
            RecipeBookChangeSettingsCommand {
                book_type: RecipeBookType::Crafting,
                open: true,
                filtering: true,
            },
        ))
        .await
        .unwrap();
        tx.send(NetCommand::RecipeBookSeenRecipe(
            RecipeBookSeenRecipeCommand {
                recipe: RecipeDisplayId { index: 321 },
            },
        ))
        .await
        .unwrap();
        tx.send(NetCommand::Disconnect).await.unwrap();
        let mut player_position_state = PlayerPositionState::default();

        drive_play_until_disconnect(&mut conn, &mut commands, &mut player_position_state).await;

        let (packet_id, payload) =
            read_server_packet(&mut server, "recipe book change settings").await;
        assert_eq!(
            packet_id,
            ids::play::SERVERBOUND_RECIPE_BOOK_CHANGE_SETTINGS
        );
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 0);
        assert!(decoder.read_bool().unwrap());
        assert!(decoder.read_bool().unwrap());
        assert!(decoder.is_empty());

        let (packet_id, payload) = read_server_packet(&mut server, "recipe book seen recipe").await;
        assert_eq!(packet_id, ids::play::SERVERBOUND_RECIPE_BOOK_SEEN_RECIPE);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 321);
        assert!(decoder.is_empty());
    }

    #[tokio::test]
    async fn drive_connection_sends_sign_update_net_command_in_play() {
        let (mut conn, mut server) = raw_connection_pair_with_server().await;
        let (tx, mut commands) = mpsc::channel(2);
        tx.send(NetCommand::SignUpdate(SignUpdate {
            pos: BlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            is_front_text: false,
            lines: [
                "line 0".to_string(),
                "line 1".to_string(),
                "line 2".to_string(),
                "line 3".to_string(),
            ],
        }))
        .await
        .unwrap();
        tx.send(NetCommand::Disconnect).await.unwrap();
        let mut player_position_state = PlayerPositionState::default();

        drive_play_until_disconnect(&mut conn, &mut commands, &mut player_position_state).await;

        let (packet_id, payload) = read_server_packet(&mut server, "sign update").await;
        assert_eq!(packet_id, ids::play::SERVERBOUND_SIGN_UPDATE);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(
            decode_packed_block_pos(decoder.read_i64().unwrap()),
            BlockPos {
                x: -5,
                y: 70,
                z: 12,
            }
        );
        assert!(!decoder.read_bool().unwrap());
        assert_eq!(decoder.read_string(384).unwrap(), "line 0");
        assert_eq!(decoder.read_string(384).unwrap(), "line 1");
        assert_eq!(decoder.read_string(384).unwrap(), "line 2");
        assert_eq!(decoder.read_string(384).unwrap(), "line 3");
        assert!(decoder.is_empty());
    }

    #[tokio::test]
    async fn drive_connection_sends_ping_request_net_command_in_play() {
        let (mut conn, mut server) = raw_connection_pair_with_server().await;
        let (tx, mut commands) = mpsc::channel(2);
        tx.send(NetCommand::PingRequest(123_456_789)).await.unwrap();
        tx.send(NetCommand::Disconnect).await.unwrap();
        let mut player_position_state = PlayerPositionState::default();

        drive_play_until_disconnect(&mut conn, &mut commands, &mut player_position_state).await;

        let (packet_id, payload) = read_server_packet(&mut server, "ping request").await;
        assert_eq!(packet_id, ids::play::SERVERBOUND_PING_REQUEST);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_i64().unwrap(), 123_456_789);
        assert!(decoder.is_empty());
    }

    #[tokio::test]
    async fn drive_connection_sends_select_trade_net_command_in_play() {
        let (mut conn, mut server) = raw_connection_pair_with_server().await;
        let (tx, mut commands) = mpsc::channel(2);
        tx.send(NetCommand::SelectTrade(SelectTradeCommand { item: 2 }))
            .await
            .unwrap();
        tx.send(NetCommand::Disconnect).await.unwrap();
        let mut player_position_state = PlayerPositionState::default();

        drive_play_until_disconnect(&mut conn, &mut commands, &mut player_position_state).await;

        let (packet_id, payload) = read_server_packet(&mut server, "select trade").await;
        assert_eq!(packet_id, ids::play::SERVERBOUND_SELECT_TRADE);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 2);
        assert!(decoder.is_empty());
    }

    #[tokio::test]
    async fn drive_connection_sends_interaction_net_commands_in_play() {
        let (mut conn, mut server) = raw_connection_pair_with_server().await;
        let (tx, mut commands) = mpsc::channel(10);
        tx.send(NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            pos: BlockPos { x: 1, y: 64, z: -2 },
            direction: Direction::West,
            sequence: 9,
        }))
        .await
        .unwrap();
        tx.send(NetCommand::AttackEntity(AttackEntity { entity_id: 123 }))
            .await
            .unwrap();
        tx.send(NetCommand::InteractEntity(InteractEntity {
            entity_id: 5,
            hand: InteractionHand::OffHand,
            location: Vec3d::default(),
            using_secondary_action: true,
        }))
        .await
        .unwrap();
        tx.send(NetCommand::Swing(InteractionHand::MainHand))
            .await
            .unwrap();
        tx.send(NetCommand::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos: BlockPos {
                    x: -5,
                    y: 70,
                    z: 12,
                },
                direction: Direction::South,
                cursor_x: 0.25,
                cursor_y: 0.5,
                cursor_z: 0.75,
                inside: false,
                world_border_hit: false,
            },
            sequence: 4,
        }))
        .await
        .unwrap();
        tx.send(NetCommand::UseItem(UseItem {
            hand: InteractionHand::OffHand,
            sequence: 8,
            y_rot: 45.0,
            x_rot: -20.0,
        }))
        .await
        .unwrap();
        tx.send(NetCommand::PickItemFromBlock(PickItemFromBlock {
            pos: BlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            include_data: true,
        }))
        .await
        .unwrap();
        tx.send(NetCommand::PickItemFromEntity(PickItemFromEntity {
            entity_id: 456,
            include_data: false,
        }))
        .await
        .unwrap();
        tx.send(NetCommand::PaddleBoat(PaddleBoat {
            left: true,
            right: false,
        }))
        .await
        .unwrap();
        tx.send(NetCommand::Disconnect).await.unwrap();
        let mut player_position_state = PlayerPositionState::default();

        drive_play_until_disconnect(&mut conn, &mut commands, &mut player_position_state).await;

        let (packet_id, payload) = read_server_packet(&mut server, "player action").await;
        assert_eq!(packet_id, ids::play::SERVERBOUND_PLAYER_ACTION);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 0);
        assert_eq!(
            decode_packed_block_pos(decoder.read_i64().unwrap()),
            BlockPos { x: 1, y: 64, z: -2 }
        );
        assert_eq!(decoder.read_u8().unwrap(), 4);
        assert_eq!(decoder.read_var_i32().unwrap(), 9);
        assert!(decoder.is_empty());

        let (packet_id, payload) = read_server_packet(&mut server, "attack entity").await;
        assert_eq!(packet_id, ids::play::SERVERBOUND_ATTACK);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 123);
        assert!(decoder.is_empty());

        let (packet_id, payload) = read_server_packet(&mut server, "interact entity").await;
        assert_eq!(packet_id, ids::play::SERVERBOUND_INTERACT);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 5);
        assert_eq!(decoder.read_var_i32().unwrap(), 1);
        assert_eq!(decoder.read_u8().unwrap(), 0);
        assert!(decoder.read_bool().unwrap());
        assert!(decoder.is_empty());

        let (packet_id, payload) = read_server_packet(&mut server, "swing hand").await;
        assert_eq!(packet_id, ids::play::SERVERBOUND_SWING);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 0);
        assert!(decoder.is_empty());

        let (packet_id, payload) = read_server_packet(&mut server, "use item on").await;
        assert_eq!(packet_id, ids::play::SERVERBOUND_USE_ITEM_ON);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 0);
        assert_eq!(
            decode_packed_block_pos(decoder.read_i64().unwrap()),
            BlockPos {
                x: -5,
                y: 70,
                z: 12,
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

        let (packet_id, payload) = read_server_packet(&mut server, "use item").await;
        assert_eq!(packet_id, ids::play::SERVERBOUND_USE_ITEM);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 1);
        assert_eq!(decoder.read_var_i32().unwrap(), 8);
        assert_eq!(decoder.read_f32().unwrap(), 45.0);
        assert_eq!(decoder.read_f32().unwrap(), -20.0);
        assert!(decoder.is_empty());

        let (packet_id, payload) = read_server_packet(&mut server, "pick item from block").await;
        assert_eq!(packet_id, ids::play::SERVERBOUND_PICK_ITEM_FROM_BLOCK);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(
            decode_packed_block_pos(decoder.read_i64().unwrap()),
            BlockPos {
                x: -5,
                y: 70,
                z: 12,
            }
        );
        assert!(decoder.read_bool().unwrap());
        assert!(decoder.is_empty());

        let (packet_id, payload) = read_server_packet(&mut server, "pick item from entity").await;
        assert_eq!(packet_id, ids::play::SERVERBOUND_PICK_ITEM_FROM_ENTITY);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 456);
        assert!(!decoder.read_bool().unwrap());
        assert!(decoder.is_empty());

        let (packet_id, payload) = read_server_packet(&mut server, "paddle boat").await;
        assert_eq!(packet_id, ids::play::SERVERBOUND_PADDLE_BOAT);
        let mut decoder = Decoder::new(&payload);
        assert!(decoder.read_bool().unwrap());
        assert!(!decoder.read_bool().unwrap());
        assert!(decoder.is_empty());
    }

    #[tokio::test]
    async fn drive_connection_sends_inventory_net_commands_in_play() {
        let (mut conn, mut server) = raw_connection_pair_with_server().await;
        let (tx, mut commands) = mpsc::channel(7);
        tx.send(NetCommand::SetHeldSlot(12)).await.unwrap();
        tx.send(NetCommand::SelectBundleItem(SelectBundleItem {
            slot_id: 9,
            selected_item_index: -1,
        }))
        .await
        .unwrap();
        tx.send(NetCommand::ContainerButtonClick(ContainerButtonClick {
            container_id: 7,
            button_id: 2,
        }))
        .await
        .unwrap();
        tx.send(NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 33,
            slot_num: 5,
            button_num: 1,
            input: ContainerInput::Pickup,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        }))
        .await
        .unwrap();
        tx.send(NetCommand::ContainerClose(ContainerCloseRequest {
            container_id: 7,
        }))
        .await
        .unwrap();
        tx.send(NetCommand::ContainerSlotStateChanged(
            ContainerSlotStateChanged {
                slot_id: 12,
                container_id: 7,
                new_state: true,
            },
        ))
        .await
        .unwrap();
        tx.send(NetCommand::Disconnect).await.unwrap();
        let mut player_position_state = PlayerPositionState::default();

        drive_play_until_disconnect(&mut conn, &mut commands, &mut player_position_state).await;

        let (packet_id, payload) = read_server_packet(&mut server, "set carried item").await;
        assert_eq!(packet_id, ids::play::SERVERBOUND_SET_CARRIED_ITEM);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_i16().unwrap(), 8);
        assert!(decoder.is_empty());

        let (packet_id, payload) = read_server_packet(&mut server, "select bundle item").await;
        assert_eq!(packet_id, ids::play::SERVERBOUND_BUNDLE_ITEM_SELECTED);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 9);
        assert_eq!(decoder.read_var_i32().unwrap(), -1);
        assert!(decoder.is_empty());

        let (packet_id, payload) = read_server_packet(&mut server, "container button click").await;
        assert_eq!(packet_id, ids::play::SERVERBOUND_CONTAINER_BUTTON_CLICK);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 7);
        assert_eq!(decoder.read_var_i32().unwrap(), 2);
        assert!(decoder.is_empty());

        let (packet_id, payload) = read_server_packet(&mut server, "container click").await;
        assert_eq!(packet_id, ids::play::SERVERBOUND_CONTAINER_CLICK);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 7);
        assert_eq!(decoder.read_var_i32().unwrap(), 33);
        assert_eq!(decoder.read_i16().unwrap(), 5);
        assert_eq!(decoder.read_i8().unwrap(), 1);
        assert_eq!(decoder.read_var_i32().unwrap(), 0);
        assert_eq!(decoder.read_var_i32().unwrap(), 0);
        assert!(!decoder.read_bool().unwrap());
        assert!(decoder.is_empty());

        let (packet_id, payload) = read_server_packet(&mut server, "container close").await;
        assert_eq!(packet_id, ids::play::SERVERBOUND_CONTAINER_CLOSE);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 7);
        assert!(decoder.is_empty());

        let (packet_id, payload) =
            read_server_packet(&mut server, "container slot state changed").await;
        assert_eq!(
            packet_id,
            ids::play::SERVERBOUND_CONTAINER_SLOT_STATE_CHANGED
        );
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 12);
        assert_eq!(decoder.read_var_i32().unwrap(), 7);
        assert!(decoder.read_bool().unwrap());
        assert!(decoder.is_empty());
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

    async fn raw_connection_pair_with_server() -> (RawConnection, RawConnection) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let client = tokio::spawn(async move {
            RawConnection::connect(&addr.to_string(), None)
                .await
                .unwrap()
        });
        let (server_stream, _) = listener.accept().await.unwrap();
        let client = client.await.unwrap();
        let server = RawConnection {
            stream: server_stream,
            read_buf: BytesMut::new(),
            compression_threshold: None,
        };
        (client, server)
    }

    async fn raw_connection_pair_expect_accept_code_of_conduct(
    ) -> (RawConnection, tokio::task::JoinHandle<()>) {
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
                .expect("code-of-conduct accept should be sent")
                .unwrap();
            assert_eq!(
                packet_id,
                ids::configuration::SERVERBOUND_ACCEPT_CODE_OF_CONDUCT
            );
            assert!(payload.is_empty());
        });
        let conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();
        (conn, server)
    }

    async fn drive_play_until_disconnect(
        conn: &mut RawConnection,
        commands: &mut mpsc::Receiver<NetCommand>,
        player_position_state: &mut PlayerPositionState,
    ) {
        let mut play_tick = Some(dormant_play_tick());
        let result = timeout(
            Duration::from_secs(1),
            read_packet_or_drive_connection(
                conn,
                ConnectionState::Play,
                &mut play_tick,
                commands,
                player_position_state,
            ),
        )
        .await
        .expect("drive should not hang")
        .unwrap();
        assert!(matches!(result, ConnectionDrive::Disconnect));
    }

    fn dormant_play_tick() -> Interval {
        interval_at(
            TokioInstant::now() + Duration::from_secs(60),
            Duration::from_secs(60),
        )
    }

    async fn read_server_packet(conn: &mut RawConnection, label: &str) -> (i32, Vec<u8>) {
        timeout(Duration::from_secs(1), conn.read_packet())
            .await
            .unwrap_or_else(|_| panic!("{label} packet should be sent"))
            .unwrap()
    }

    fn decode_packed_block_pos(packed: i64) -> BlockPos {
        BlockPos {
            x: (packed >> 38) as i32,
            y: ((packed << 52) >> 52) as i32,
            z: ((packed << 26) >> 38) as i32,
        }
    }
}
