use super::*;

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use crate::{
    chunk_batch::ChunkBatchSizeCalculator, driver::read_packet_or_drive_connection,
    session::remote_disconnect_error,
};
use bbb_protocol::{
    code_of_conduct_text_hash,
    codec::{Decoder, Encoder},
    content_registry::{
        ContentRegistry, ContentRegistryBuilder, ModBlockRegistration, BBB_CONTENT_REGISTRY_CHANNEL,
    },
    entity_synched_data::{
        VANILLA_ENTITY_AIR_SUPPLY_DATA_ID, VANILLA_ENTITY_SHARED_FLAGS_DATA_ID,
        VANILLA_ENTITY_SHARED_FLAG_INVISIBLE,
    },
    entity_types::VANILLA_ENTITY_TYPE_AXOLOTL_ID,
    frame::encode_packet,
    ids,
    mob_effect_types::VANILLA_MOB_EFFECT_MINING_FATIGUE_ID,
    packets::{
        ChatAcknowledgement, CustomPayloadBody, GameProfile, LoginClientbound, PlayClientbound,
        PlayerPositionState,
    },
    particle_types::VANILLA_PARTICLE_TYPE_DAMAGE_INDICATOR_ID,
};

use tokio::{net::TcpListener, time::timeout};

fn entity_invisible_data() -> packets::EntityDataValue {
    packets::EntityDataValue {
        data_id: VANILLA_ENTITY_SHARED_FLAGS_DATA_ID,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        value: packets::EntityDataValueKind::Byte(VANILLA_ENTITY_SHARED_FLAG_INVISIBLE),
    }
}

fn entity_air_supply_data(air_supply: i32) -> packets::EntityDataValue {
    packets::EntityDataValue {
        data_id: VANILLA_ENTITY_AIR_SUPPLY_DATA_ID,
        serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        value: packets::EntityDataValueKind::Int(air_supply),
    }
}

async fn raw_connection_pair() -> (RawConnection, RawConnection) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let client = tokio::spawn(async move {
        RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap()
    });
    let (server_stream, _) = listener.accept().await.unwrap();
    let client = client.await.unwrap();
    let server = RawConnection::from_stream(server_stream, None);
    (client, server)
}

fn encoded_store_cookie_packet(packet_id: i32, key: &str, payload: &[u8]) -> Vec<u8> {
    let mut encoder = Encoder::new();
    encoder.write_string(key);
    encoder.write_var_i32(payload.len() as i32);
    encoder.write_bytes(payload);
    encode_packet(packet_id, &encoder.into_inner())
}

fn encoded_transfer_packet(packet_id: i32, host: &str, port: i32) -> Vec<u8> {
    let mut encoder = Encoder::new();
    encoder.write_string(host);
    encoder.write_var_i32(port);
    encode_packet(packet_id, &encoder.into_inner())
}

fn test_item_stack(item_id: i32, count: i32) -> packets::ItemStackSummary {
    packets::ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: packets::DataComponentPatchSummary::default().into(),
    }
}

fn command_tree_packet(literal: &str) -> packets::Commands {
    packets::Commands {
        root_index: 0,
        nodes: vec![
            packets::CommandNode {
                node_type: packets::CommandNodeType::Root,
                flags: 0,
                children: vec![1],
                redirect: None,
                name: None,
                parser: None,
                suggestions: None,
                executable: false,
                restricted: false,
            },
            packets::CommandNode {
                node_type: packets::CommandNodeType::Literal,
                flags: 1,
                children: vec![2],
                redirect: None,
                name: Some(literal.to_string()),
                parser: None,
                suggestions: None,
                executable: false,
                restricted: false,
            },
            packets::CommandNode {
                node_type: packets::CommandNodeType::Argument,
                flags: 54,
                children: Vec::new(),
                redirect: None,
                name: Some("message".to_string()),
                parser: Some(packets::CommandArgumentParser {
                    type_id: packets::VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID,
                    name: "minecraft:message".to_string(),
                    properties: Vec::new(),
                }),
                suggestions: Some("minecraft:ask_server".to_string()),
                executable: true,
                restricted: true,
            },
        ],
    }
}

fn player_chat_packet(global_index: i32) -> packets::PlayerChat {
    packets::PlayerChat {
        global_index,
        sender: uuid::Uuid::from_u128(0x1234),
        index: global_index,
        signature: None,
        body: packets::SignedMessageBody {
            content: format!("message {global_index}"),
            timestamp_millis: i64::from(global_index),
            salt: i64::from(global_index) + 1,
            last_seen: Vec::new(),
        },
        unsigned_content: None,
        unsigned_content_styled: None,
        unsigned_content_component: None,
        filter_mask: packets::FilterMask {
            kind: packets::FilterMaskKind::PassThrough,
            mask_words: Vec::new(),
        },
        chat_type: chat_type_bound("Alice"),
    }
}

fn chat_type_bound(name: &str) -> packets::ChatTypeBound {
    packets::ChatTypeBound {
        chat_type: packets::ChatTypeHolder::Registry { id: 0 },
        name: name.to_string(),
        name_styled: Vec::new(),
        name_component: None,
        target_name: None,
        target_name_styled: None,
        target_name_component: None,
    }
}

fn assert_move_player_pos_rot_payload(
    payload: &[u8],
    x: f64,
    y: f64,
    z: f64,
    y_rot: f32,
    x_rot: f32,
    flags: u8,
) {
    let mut decoder = Decoder::new(payload);
    assert_eq!(decoder.read_f64().unwrap(), x);
    assert_eq!(decoder.read_f64().unwrap(), y);
    assert_eq!(decoder.read_f64().unwrap(), z);
    assert_eq!(decoder.read_f32().unwrap(), y_rot);
    assert_eq!(decoder.read_f32().unwrap(), x_rot);
    assert_eq!(decoder.read_u8().unwrap(), flags);
    assert!(decoder.is_empty());
}

fn assert_move_player_rot_payload(payload: &[u8], y_rot: f32, x_rot: f32, flags: u8) {
    let mut decoder = Decoder::new(payload);
    assert_eq!(decoder.read_f32().unwrap(), y_rot);
    assert_eq!(decoder.read_f32().unwrap(), x_rot);
    assert_eq!(decoder.read_u8().unwrap(), flags);
    assert!(decoder.is_empty());
}

mod batch_01;
mod batch_02;

#[tokio::test]
async fn offline_login_proceeds_immediately_without_waiting_on_services_public_keys() {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use rsa::{pkcs8::EncodePublicKey, BigUint, RsaPublicKey};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let (release_keys_tx, release_keys_rx) = tokio::sync::oneshot::channel::<()>();
    let services_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let services_addr = services_listener.local_addr().unwrap();
    let modulus = (BigUint::from(1u8) << 4095) + BigUint::from(1u8);
    let der = RsaPublicKey::new(modulus, BigUint::from(65_537u32))
        .unwrap()
        .to_public_key_der()
        .unwrap()
        .as_bytes()
        .to_vec();
    let expected_der = der.clone();
    let services_server = tokio::spawn(async move {
        let (mut socket, _) = services_listener.accept().await.unwrap();
        let mut request = Vec::new();
        let mut chunk = [0u8; 1024];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            let read = socket.read(&mut chunk).await.unwrap();
            if read == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..read]);
        }
        // Hold the Services HTTP response until AFTER the game server has already
        // received TCP connect + Handshake + LoginHello!
        let _ = release_keys_rx.await;
        let body = serde_json::json!({
            "profilePropertyKeys": [],
            "playerCertificateKeys": [{"publicKey": STANDARD.encode(&der)}],
        })
        .to_string();
        socket
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    });

    let game_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let game_addr = game_listener.local_addr().unwrap();
    let (event_tx, mut event_rx) = mpsc::channel(32);
    let sender = NetEventSender::new(event_tx, || {});
    let (command_tx, command_rx) = mpsc::channel(8);

    let options = ConnectionOptions::offline(game_addr.to_string(), "OfflineFast").unwrap();
    let endpoint = format!("http://{services_addr}/publickeys");
    let client_task = tokio::spawn(async move {
        run_event_stream_with_services_endpoint(options, sender, command_rx, endpoint).await
    });

    // Accept TCP + read Handshake + LoginHello well under 500ms even though Services HTTP is stalled.
    let (server_stream, _) = timeout(Duration::from_millis(500), game_listener.accept())
        .await
        .expect("TCP connect must not wait on Services HTTPS")
        .unwrap();
    let mut server = RawConnection::from_stream(server_stream, None);
    let (handshake_id, _) = timeout(Duration::from_millis(500), server.read_packet())
        .await
        .expect("Handshake must arrive without Services HTTPS stall")
        .unwrap();
    assert_eq!(handshake_id, 0x00);
    let (login_hello_id, _) = timeout(Duration::from_millis(500), server.read_packet())
        .await
        .expect("LoginHello must arrive without Services HTTPS stall")
        .unwrap();
    assert_eq!(login_hello_id, 0x00);

    // Release the delayed Services HTTP response and observe background key arrival.
    let _ = release_keys_tx.send(());
    let mut saw_keys = false;
    while !saw_keys {
        let ev = timeout(Duration::from_secs(2), event_rx.recv())
            .await
            .expect("ServicesPublicKeys event should arrive asynchronously")
            .unwrap();
        if let NetEvent::ServicesPublicKeys {
            profile_property_keys,
            player_certificate_keys,
        } = ev
        {
            assert!(profile_property_keys.is_empty());
            assert_eq!(player_certificate_keys, vec![expected_der.clone()]);
            saw_keys = true;
        }
    }

    command_tx.send(NetCommand::Disconnect).await.unwrap();
    let exit = timeout(Duration::from_secs(2), client_task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(exit, EventStreamExit::LocalDisconnect);
    services_server.await.unwrap();
}
