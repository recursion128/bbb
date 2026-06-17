use super::{
    maybe_send_perform_respawn, send_accept_code_of_conduct, send_attack_entity, send_chat_command,
    send_command_suggestion_request, send_container_button_click, send_container_click,
    send_container_close, send_container_slot_state_changed, send_edit_book, send_interact_entity,
    send_paddle_boat, send_pick_item_from_block, send_pick_item_from_entity, send_ping_request,
    send_place_recipe, send_player_abilities_command, send_player_action, send_player_command,
    send_player_input_command, send_player_move_command, send_recipe_book_change_settings,
    send_recipe_book_seen_recipe, send_rename_item, send_seen_advancements,
    send_select_bundle_item, send_select_trade, send_set_beacon, send_set_held_slot_command,
    send_sign_update, send_swing_command, send_use_item, send_use_item_on,
};
use crate::{
    connection::RawConnection,
    types::{PlayerMoveCommand, VehicleMoveCommand},
};
use bbb_protocol::{
    codec::Decoder,
    ids,
    packets::{
        AttackEntity, BlockPos, ChatCommand, CommandSuggestionRequest, ContainerButtonClick,
        ContainerClick, ContainerCloseRequest, ContainerInput, ContainerSlotStateChanged, EditBook,
        HashedComponentPatch, HashedItemStack, HashedStack, InteractEntity, InteractionHand,
        PaddleBoat, PickItemFromEntity, PlaceRecipeCommand, PlayerAbilitiesCommand, PlayerAction,
        PlayerCommand, PlayerHealth, PlayerInput, PlayerPositionState,
        RecipeBookChangeSettingsCommand, RecipeBookSeenRecipeCommand, RecipeBookType,
        RecipeDisplayId, RenameItem, SeenAdvancements, SelectBundleItem, SelectTradeCommand,
        SetBeacon, SignUpdate, Vec3d,
    },
};
use bytes::BytesMut;
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};
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
        force_position: false,
    };

    let (packet_id, payload) = command.encode_packet_from(PlayerPositionState::default());

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
fn player_move_command_selects_vanilla_move_player_variant_from_previous_state() {
    let previous = PlayerPositionState {
        position: Vec3d {
            x: 1.25,
            y: 64.5,
            z: -8.75,
        },
        delta_movement: Vec3d::default(),
        y_rot: 90.0,
        x_rot: -15.0,
    };

    let (packet_id, payload) = PlayerMoveCommand {
        state: PlayerPositionState {
            position: Vec3d {
                x: 2.0,
                ..previous.position
            },
            ..previous
        },
        on_ground: true,
        horizontal_collision: false,
        force_position: false,
    }
    .encode_packet_from(previous);
    assert_eq!(packet_id, ids::play::SERVERBOUND_MOVE_PLAYER_POS);
    assert_eq!(payload.len(), 25);

    let (packet_id, payload) = PlayerMoveCommand {
        state: PlayerPositionState {
            y_rot: 100.0,
            x_rot: 5.0,
            ..previous
        },
        on_ground: false,
        horizontal_collision: true,
        force_position: false,
    }
    .encode_packet_from(previous);
    assert_eq!(packet_id, ids::play::SERVERBOUND_MOVE_PLAYER_ROT);
    assert_eq!(payload.len(), 9);

    let (packet_id, payload) = PlayerMoveCommand {
        state: PlayerPositionState {
            position: Vec3d {
                z: -9.25,
                ..previous.position
            },
            y_rot: 100.0,
            ..previous
        },
        on_ground: true,
        horizontal_collision: true,
        force_position: false,
    }
    .encode_packet_from(previous);
    assert_eq!(packet_id, ids::play::SERVERBOUND_MOVE_PLAYER_POS_ROT);
    assert_eq!(payload.len(), 33);

    let (packet_id, payload) = PlayerMoveCommand {
        state: previous,
        on_ground: false,
        horizontal_collision: true,
        force_position: false,
    }
    .encode_packet_from(previous);
    assert_eq!(packet_id, ids::play::SERVERBOUND_MOVE_PLAYER_STATUS_ONLY);
    assert_eq!(payload, vec![0b10]);

    let (packet_id, payload) = PlayerMoveCommand {
        state: previous,
        on_ground: false,
        horizontal_collision: true,
        force_position: true,
    }
    .encode_packet_from(previous);
    assert_eq!(packet_id, ids::play::SERVERBOUND_MOVE_PLAYER_POS);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_f64().unwrap(), previous.position.x);
    assert_eq!(decoder.read_f64().unwrap(), previous.position.y);
    assert_eq!(decoder.read_f64().unwrap(), previous.position.z);
    assert_eq!(decoder.read_u8().unwrap(), 0b10);
    assert!(decoder.is_empty());
}

#[tokio::test]
async fn send_player_move_command_selects_variant_and_updates_position_state() {
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
            .expect("move player command should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_MOVE_PLAYER_POS);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_f64().unwrap(), 2.0);
        assert_eq!(decoder.read_f64().unwrap(), 64.5);
        assert_eq!(decoder.read_f64().unwrap(), -8.75);
        assert_eq!(decoder.read_u8().unwrap(), 0b01);
        assert!(decoder.is_empty());
    });
    let mut conn = RawConnection::connect(&addr.to_string(), None)
        .await
        .unwrap();
    let mut player_position_state = PlayerPositionState {
        position: Vec3d {
            x: 1.25,
            y: 64.5,
            z: -8.75,
        },
        delta_movement: Vec3d::default(),
        y_rot: 90.0,
        x_rot: -15.0,
    };
    let next = PlayerPositionState {
        position: Vec3d {
            x: 2.0,
            ..player_position_state.position
        },
        ..player_position_state
    };

    send_player_move_command(
        &mut conn,
        PlayerMoveCommand {
            state: next,
            on_ground: true,
            horizontal_collision: false,
            force_position: false,
        },
        &mut player_position_state,
    )
    .await
    .unwrap();

    assert_eq!(player_position_state, next);
    server.await.unwrap();
}

fn decode_packed_block_pos(packed: i64) -> BlockPos {
    BlockPos {
        x: (packed >> 38) as i32,
        y: (packed << 52 >> 52) as i32,
        z: (packed << 26 >> 38) as i32,
    }
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
async fn send_accept_code_of_conduct_encodes_configuration_accept_packet() {
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
    let mut conn = RawConnection::connect(&addr.to_string(), None)
        .await
        .unwrap();

    send_accept_code_of_conduct(&mut conn).await.unwrap();

    server.await.unwrap();
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
async fn send_chat_command_encodes_chat_command_packet() {
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
            .expect("chat command should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_CHAT_COMMAND);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(
            decoder.read_string(32767).unwrap(),
            "give @p minecraft:stone"
        );
        assert!(decoder.is_empty());
    });
    let mut conn = RawConnection::connect(&addr.to_string(), None)
        .await
        .unwrap();

    send_chat_command(
        &mut conn,
        ChatCommand {
            command: "give @p minecraft:stone".to_string(),
        },
    )
    .await
    .unwrap();

    server.await.unwrap();
}

#[tokio::test]
async fn send_entity_interaction_commands_encode_packets() {
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
            .expect("attack entity command should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_ATTACK);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 123);
        assert!(decoder.is_empty());

        let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
            .await
            .expect("interact entity command should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_INTERACT);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 5);
        assert_eq!(decoder.read_var_i32().unwrap(), 0);
        assert_eq!(
            decoder.read_exact(6, "lp_vec3").unwrap(),
            &[0xf1, 0xff, 0x00, 0x00, 0xff, 0xff]
        );
        assert!(!decoder.read_bool().unwrap());
        assert!(decoder.is_empty());
    });
    let mut conn = RawConnection::connect(&addr.to_string(), None)
        .await
        .unwrap();

    send_attack_entity(&mut conn, AttackEntity { entity_id: 123 })
        .await
        .unwrap();
    send_interact_entity(
        &mut conn,
        InteractEntity {
            entity_id: 5,
            hand: InteractionHand::MainHand,
            location: Vec3d {
                x: 1.0,
                y: 0.0,
                z: -1.0,
            },
            using_secondary_action: false,
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
async fn send_player_abilities_command_encodes_flying_bit() {
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
            .expect("player abilities command should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_PLAYER_ABILITIES);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_u8().unwrap(), 0x02);
        assert!(decoder.is_empty());
    });
    let mut conn = RawConnection::connect(&addr.to_string(), None)
        .await
        .unwrap();

    send_player_abilities_command(&mut conn, PlayerAbilitiesCommand { flying: true })
        .await
        .unwrap();

    server.await.unwrap();
}

#[tokio::test]
async fn send_place_recipe_encodes_place_recipe_packet() {
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
            .expect("place recipe command should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_PLACE_RECIPE);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 7);
        assert_eq!(decoder.read_var_i32().unwrap(), 123);
        assert!(decoder.read_bool().unwrap());
        assert!(decoder.is_empty());
    });
    let mut conn = RawConnection::connect(&addr.to_string(), None)
        .await
        .unwrap();

    send_place_recipe(
        &mut conn,
        PlaceRecipeCommand {
            container_id: 7,
            recipe_index: 123,
            use_max_items: true,
        },
    )
    .await
    .unwrap();

    server.await.unwrap();
}

#[tokio::test]
async fn send_select_trade_encodes_select_trade_packet() {
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
            .expect("select trade command should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_SELECT_TRADE);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 2);
        assert!(decoder.is_empty());
    });
    let mut conn = RawConnection::connect(&addr.to_string(), None)
        .await
        .unwrap();

    send_select_trade(&mut conn, SelectTradeCommand { item: 2 })
        .await
        .unwrap();

    server.await.unwrap();
}

#[tokio::test]
async fn send_select_bundle_item_encodes_bundle_item_selected_packet() {
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
            .expect("select bundle item command should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_BUNDLE_ITEM_SELECTED);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 7);
        assert_eq!(decoder.read_var_i32().unwrap(), -1);
        assert!(decoder.is_empty());
    });
    let mut conn = RawConnection::connect(&addr.to_string(), None)
        .await
        .unwrap();

    send_select_bundle_item(
        &mut conn,
        SelectBundleItem {
            slot_id: 7,
            selected_item_index: -1,
        },
    )
    .await
    .unwrap();

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
async fn send_pick_item_from_entity_encodes_pick_packet() {
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
            .expect("pick item from entity command should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_PICK_ITEM_FROM_ENTITY);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 123);
        assert!(decoder.read_bool().unwrap());
        assert!(decoder.is_empty());
    });
    let mut conn = RawConnection::connect(&addr.to_string(), None)
        .await
        .unwrap();

    send_pick_item_from_entity(
        &mut conn,
        PickItemFromEntity {
            entity_id: 123,
            include_data: true,
        },
    )
    .await
    .unwrap();

    server.await.unwrap();
}

#[tokio::test]
async fn send_paddle_boat_encodes_left_and_right_flags() {
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
            .expect("paddle boat command should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_PADDLE_BOAT);
        let mut decoder = Decoder::new(&payload);
        assert!(decoder.read_bool().unwrap());
        assert!(!decoder.read_bool().unwrap());
        assert!(decoder.is_empty());
    });
    let mut conn = RawConnection::connect(&addr.to_string(), None)
        .await
        .unwrap();

    send_paddle_boat(
        &mut conn,
        PaddleBoat {
            left: true,
            right: false,
        },
    )
    .await
    .unwrap();

    server.await.unwrap();
}

#[tokio::test]
async fn send_ping_request_encodes_play_ping_request_packet() {
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
            .expect("ping request command should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_PING_REQUEST);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_i64().unwrap(), 123_456_789);
        assert!(decoder.is_empty());
    });
    let mut conn = RawConnection::connect(&addr.to_string(), None)
        .await
        .unwrap();

    send_ping_request(&mut conn, 123_456_789).await.unwrap();

    server.await.unwrap();
}

#[tokio::test]
async fn send_recipe_book_commands_encode_packets() {
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
            .expect("recipe book settings command should be sent")
            .unwrap();
        assert_eq!(
            packet_id,
            ids::play::SERVERBOUND_RECIPE_BOOK_CHANGE_SETTINGS
        );
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 3);
        assert!(decoder.read_bool().unwrap());
        assert!(!decoder.read_bool().unwrap());
        assert!(decoder.is_empty());

        let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
            .await
            .expect("recipe book seen recipe command should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_RECIPE_BOOK_SEEN_RECIPE);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 321);
        assert!(decoder.is_empty());
    });
    let mut conn = RawConnection::connect(&addr.to_string(), None)
        .await
        .unwrap();

    send_recipe_book_change_settings(
        &mut conn,
        RecipeBookChangeSettingsCommand {
            book_type: RecipeBookType::Smoker,
            open: true,
            filtering: false,
        },
    )
    .await
    .unwrap();
    send_recipe_book_seen_recipe(
        &mut conn,
        RecipeBookSeenRecipeCommand {
            recipe: RecipeDisplayId { index: 321 },
        },
    )
    .await
    .unwrap();

    server.await.unwrap();
}

#[tokio::test]
async fn send_edit_book_encodes_edit_book_packet() {
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
            .expect("edit book command should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_EDIT_BOOK);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 5);
        assert_eq!(decoder.read_var_i32().unwrap(), 2);
        assert_eq!(decoder.read_string(1024).unwrap(), "first page");
        assert_eq!(decoder.read_string(1024).unwrap(), "second page");
        assert!(decoder.read_bool().unwrap());
        assert_eq!(decoder.read_string(32).unwrap(), "Field Notes");
        assert!(decoder.is_empty());
    });
    let mut conn = RawConnection::connect(&addr.to_string(), None)
        .await
        .unwrap();

    send_edit_book(
        &mut conn,
        EditBook {
            slot: 5,
            pages: vec!["first page".to_string(), "second page".to_string()],
            title: Some("Field Notes".to_string()),
        },
    )
    .await
    .unwrap();

    server.await.unwrap();
}

#[tokio::test]
async fn send_set_beacon_encodes_set_beacon_packet() {
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
            .expect("set beacon command should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_SET_BEACON);
        let mut decoder = Decoder::new(&payload);
        assert!(decoder.read_bool().unwrap());
        assert_eq!(decoder.read_var_i32().unwrap(), 1);
        assert!(!decoder.read_bool().unwrap());
        assert!(decoder.is_empty());
    });
    let mut conn = RawConnection::connect(&addr.to_string(), None)
        .await
        .unwrap();

    send_set_beacon(
        &mut conn,
        SetBeacon {
            primary_effect: Some(1),
            secondary_effect: None,
        },
    )
    .await
    .unwrap();

    server.await.unwrap();
}

#[tokio::test]
async fn send_rename_item_encodes_rename_item_packet() {
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
            .expect("rename item command should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_RENAME_ITEM);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_string(32767).unwrap(), "Sharp Pick");
        assert!(decoder.is_empty());
    });
    let mut conn = RawConnection::connect(&addr.to_string(), None)
        .await
        .unwrap();

    send_rename_item(
        &mut conn,
        RenameItem {
            name: "Sharp Pick".to_string(),
        },
    )
    .await
    .unwrap();

    server.await.unwrap();
}

#[tokio::test]
async fn send_seen_advancements_encodes_opened_tab_packet() {
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
            .expect("seen advancements command should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_SEEN_ADVANCEMENTS);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 0);
        assert_eq!(decoder.read_string(32767).unwrap(), "minecraft:story/root");
        assert!(decoder.is_empty());
    });
    let mut conn = RawConnection::connect(&addr.to_string(), None)
        .await
        .unwrap();

    send_seen_advancements(
        &mut conn,
        SeenAdvancements::OpenedTab {
            tab: "minecraft:story/root".to_string(),
        },
    )
    .await
    .unwrap();

    server.await.unwrap();
}

#[tokio::test]
async fn send_sign_update_encodes_sign_update_packet() {
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
            .expect("sign update command should be sent")
            .unwrap();
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
    });
    let mut conn = RawConnection::connect(&addr.to_string(), None)
        .await
        .unwrap();

    send_sign_update(
        &mut conn,
        SignUpdate {
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
        },
    )
    .await
    .unwrap();

    server.await.unwrap();
}

#[tokio::test]
async fn send_container_inventory_commands_encode_packets() {
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
            .expect("container button click should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_CONTAINER_BUTTON_CLICK);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 7);
        assert_eq!(decoder.read_var_i32().unwrap(), 2);
        assert!(decoder.is_empty());

        let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
            .await
            .expect("select bundle item should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_BUNDLE_ITEM_SELECTED);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 9);
        assert_eq!(decoder.read_var_i32().unwrap(), 2);
        assert!(decoder.is_empty());

        let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
            .await
            .expect("container click should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_CONTAINER_CLICK);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 7);
        assert_eq!(decoder.read_var_i32().unwrap(), 33);
        assert_eq!(decoder.read_i16().unwrap(), 5);
        assert_eq!(decoder.read_i8().unwrap(), 1);
        assert_eq!(decoder.read_var_i32().unwrap(), 0);
        assert_eq!(decoder.read_var_i32().unwrap(), 1);
        assert_eq!(decoder.read_i16().unwrap(), 5);
        assert!(decoder.read_bool().unwrap());
        assert_eq!(decoder.read_var_i32().unwrap(), 42);
        assert_eq!(decoder.read_var_i32().unwrap(), 64);
        assert_eq!(decoder.read_var_i32().unwrap(), 1);
        assert_eq!(decoder.read_var_i32().unwrap(), 10);
        assert_eq!(decoder.read_i32().unwrap(), 0x0102_0304);
        assert_eq!(decoder.read_var_i32().unwrap(), 1);
        assert_eq!(decoder.read_var_i32().unwrap(), 20);
        assert!(!decoder.read_bool().unwrap());
        assert!(decoder.is_empty());

        let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
            .await
            .expect("container close should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_CONTAINER_CLOSE);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 7);
        assert!(decoder.is_empty());

        let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
            .await
            .expect("container slot state should be sent")
            .unwrap();
        assert_eq!(
            packet_id,
            ids::play::SERVERBOUND_CONTAINER_SLOT_STATE_CHANGED
        );
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 12);
        assert_eq!(decoder.read_var_i32().unwrap(), 7);
        assert!(decoder.read_bool().unwrap());
        assert!(decoder.is_empty());
    });
    let mut conn = RawConnection::connect(&addr.to_string(), None)
        .await
        .unwrap();

    send_container_button_click(
        &mut conn,
        ContainerButtonClick {
            container_id: 7,
            button_id: 2,
        },
    )
    .await
    .unwrap();
    send_select_bundle_item(
        &mut conn,
        SelectBundleItem {
            slot_id: 9,
            selected_item_index: 2,
        },
    )
    .await
    .unwrap();
    let mut added_components = BTreeMap::new();
    added_components.insert(10, 0x0102_0304);
    let mut removed_components = BTreeSet::new();
    removed_components.insert(20);
    let mut changed_slots = BTreeMap::new();
    changed_slots.insert(
        5,
        HashedStack::Item(HashedItemStack {
            item_id: 42,
            count: 64,
            components: HashedComponentPatch {
                added_components,
                removed_components,
            },
        }),
    );
    send_container_click(
        &mut conn,
        ContainerClick {
            container_id: 7,
            state_id: 33,
            slot_num: 5,
            button_num: 1,
            input: ContainerInput::Pickup,
            changed_slots,
            carried_item: HashedStack::empty(),
        },
    )
    .await
    .unwrap();
    send_container_close(&mut conn, ContainerCloseRequest { container_id: 7 })
        .await
        .unwrap();
    send_container_slot_state_changed(
        &mut conn,
        ContainerSlotStateChanged {
            slot_id: 12,
            container_id: 7,
            new_state: true,
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
