use std::sync::Arc;

use bbb_protocol::{
    frame::{
        decode_packet_body, encode_packet, encode_packet_with_compression, split_packet,
        try_read_frame,
    },
    packets::{
        decode_configuration_clientbound, decode_login_clientbound, decode_pong_response,
        decode_status_response, encode_client_information,
        encode_configuration_brand_custom_payload, encode_configuration_finish, encode_handshake,
        encode_login_acknowledged, encode_login_hello, encode_ping_request,
        encode_select_known_packs, encode_status_request, ClientIntent, ConfigurationClientbound,
        LoginClientbound,
    },
    PROTOCOL_VERSION,
};
use bytes::BytesMut;
use serde_json::json;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use uuid::Uuid;

use bbb_protocol::resource_pack_response_actions::ResourcePackResponseAction;

use crate::{
    ConnectionCloseReason, ConnectionCommand, ConnectionEvent, ConnectionLimits, ConnectionPhase,
    ServerConnectionConfig, ServerListener,
};

struct TestClient {
    stream: TcpStream,
    read_buffer: BytesMut,
    compression_threshold: Option<i32>,
}

impl TestClient {
    async fn connect(address: std::net::SocketAddr) -> Self {
        Self {
            stream: TcpStream::connect(address).await.unwrap(),
            read_buffer: BytesMut::new(),
            compression_threshold: None,
        }
    }

    async fn send(&mut self, packet: (i32, Vec<u8>)) {
        let frame = encode_packet_with_compression(packet.0, &packet.1, self.compression_threshold)
            .unwrap();
        self.stream.write_all(&frame).await.unwrap();
    }

    async fn read(&mut self) -> (i32, Vec<u8>) {
        loop {
            if let Some(frame) = try_read_frame(&mut self.read_buffer).unwrap() {
                let body = decode_packet_body(&frame, self.compression_threshold).unwrap();
                let (packet_id, payload) = split_packet(&body).unwrap();
                return (packet_id, payload.to_vec());
            }
            let mut bytes = [0_u8; 4096];
            let read = self.stream.read(&mut bytes).await.unwrap();
            assert_ne!(read, 0, "server closed before the expected packet");
            self.read_buffer.extend_from_slice(&bytes[..read]);
        }
    }
}

fn config() -> Arc<ServerConnectionConfig> {
    Arc::new(ServerConnectionConfig::new(json!({
        "description": {"text": "bbb test"},
        "enforcesSecureChat": false,
        "players": {"max": 20, "online": 0},
        "version": {"name": "26.1", "protocol": PROTOCOL_VERSION}
    })))
}

#[tokio::test]
async fn loopback_status_follows_request_response_ping_pong_and_close() {
    let listener = ServerListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let mut client = TestClient::connect(address).await;
    let (_remote, _handle, driver) = listener
        .accept(config(), ConnectionLimits::default())
        .await
        .unwrap();
    let driver = tokio::spawn(driver.run());

    client
        .send(encode_handshake("localhost", address.port(), ClientIntent::Status).unwrap())
        .await;
    client.send(encode_status_request()).await;
    let (status_id, status_payload) = client.read().await;
    assert_eq!(
        status_id,
        bbb_protocol::ids::status::CLIENTBOUND_STATUS_RESPONSE
    );
    let status: serde_json::Value =
        serde_json::from_str(&decode_status_response(&status_payload).unwrap()).unwrap();
    assert_eq!(status["version"]["protocol"], PROTOCOL_VERSION);

    client.send(encode_ping_request(42)).await;
    let (pong_id, pong_payload) = client.read().await;
    assert_eq!(
        pong_id,
        bbb_protocol::ids::status::CLIENTBOUND_PONG_RESPONSE
    );
    assert_eq!(decode_pong_response(&pong_payload).unwrap(), 42);
    let outcome = driver.await.unwrap().unwrap();
    assert_eq!(outcome.final_phase, ConnectionPhase::Status);
    assert_eq!(outcome.reason, ConnectionCloseReason::StatusHandled);
}

#[tokio::test]
async fn slow_login_receives_a_disconnect_at_the_login_deadline() {
    let listener = ServerListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let mut client = TestClient::connect(address).await;
    let (_remote, _handle, driver) = listener
        .accept(config(), ConnectionLimits::default())
        .await
        .unwrap();
    let driver = tokio::spawn(driver.run());
    let started = std::time::Instant::now();
    client
        .send(encode_handshake("localhost", address.port(), ClientIntent::Login).unwrap())
        .await;
    let (id, payload) = tokio::time::timeout(std::time::Duration::from_secs(35), client.read())
        .await
        .unwrap();
    assert!(started.elapsed() >= std::time::Duration::from_secs(30));
    assert!(matches!(
        decode_login_clientbound(id, &payload).unwrap(),
        LoginClientbound::Disconnect { raw_reason, .. }
            if raw_reason.contains("multiplayer.disconnect.slow_login")
    ));
    let outcome = driver.await.unwrap().unwrap();
    assert_eq!(outcome.final_phase, ConnectionPhase::Login);
    assert_eq!(outcome.reason, ConnectionCloseReason::LoginTimeout);
}

#[tokio::test]
async fn loopback_offline_login_configuration_and_play_handoff_match_bbb_client_order() {
    let listener = ServerListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let mut client = TestClient::connect(address).await;
    let (_remote, handle, driver) = listener
        .accept(config(), ConnectionLimits::default())
        .await
        .unwrap();
    let driver = tokio::spawn(driver.run());

    client
        .send(encode_handshake("localhost", address.port(), ClientIntent::Login).unwrap())
        .await;
    client
        .send(encode_login_hello("bbb-client", Uuid::from_u128(99)).unwrap())
        .await;
    let (finished_id, finished_payload) = client.read().await;
    let LoginClientbound::LoginFinished { profile } =
        decode_login_clientbound(finished_id, &finished_payload).unwrap()
    else {
        panic!("expected login finished")
    };
    assert_eq!(profile.name, "bbb-client");
    assert_eq!(
        profile.uuid,
        bbb_protocol::codec::offline_player_uuid("bbb-client")
    );

    client.send(encode_login_acknowledged()).await;
    client
        .send(encode_configuration_brand_custom_payload("bbb-client").unwrap())
        .await;
    client
        .send(encode_client_information(&Default::default()).unwrap())
        .await;

    let (brand_id, brand_payload) = client.read().await;
    assert!(matches!(
        decode_configuration_clientbound(brand_id, &brand_payload).unwrap(),
        ConfigurationClientbound::CustomPayload(_)
    ));
    let (features_id, features_payload) = client.read().await;
    assert!(matches!(
        decode_configuration_clientbound(features_id, &features_payload).unwrap(),
        ConfigurationClientbound::UpdateEnabledFeatures(_)
    ));
    let (packs_id, packs_payload) = client.read().await;
    assert_eq!(
        decode_configuration_clientbound(packs_id, &packs_payload).unwrap(),
        ConfigurationClientbound::SelectKnownPacks {
            known_packs: Vec::new()
        }
    );
    client.send(encode_select_known_packs(&[]).unwrap()).await;

    let (tags_id, tags_payload) = client.read().await;
    assert!(matches!(
        decode_configuration_clientbound(tags_id, &tags_payload).unwrap(),
        ConfigurationClientbound::UpdateTags(_)
    ));
    let (finish_id, finish_payload) = client.read().await;
    assert_eq!(
        decode_configuration_clientbound(finish_id, &finish_payload).unwrap(),
        ConfigurationClientbound::Finish
    );
    client.send(encode_configuration_finish()).await;

    let mut saw_play_ready = false;
    for _ in 0..5 {
        match handle.next_event().await.unwrap() {
            ConnectionEvent::PlayReady {
                profile,
                client_brand,
                ..
            } => {
                assert_eq!(profile.name, "bbb-client");
                assert_eq!(client_brand.as_deref(), Some("bbb-client"));
                saw_play_ready = true;
                break;
            }
            ConnectionEvent::PhaseChanged { .. } => {}
            other => panic!("unexpected connection event: {other:?}"),
        }
    }
    assert!(saw_play_ready);

    client
        .stream
        .write_all(&encode_packet(7, b"play"))
        .await
        .unwrap();
    assert_eq!(
        handle.next_event().await,
        Some(ConnectionEvent::PlayPacket {
            packet_id: 7,
            payload: b"play".to_vec(),
        })
    );
    handle.send(ConnectionCommand::Close).await.unwrap();
    let outcome = driver.await.unwrap().unwrap();
    assert_eq!(outcome.final_phase, ConnectionPhase::Play);
    assert_eq!(outcome.reason, ConnectionCloseReason::ServerCommand);
}

#[tokio::test]
async fn compression_switches_only_after_the_login_compression_packet() {
    let listener = ServerListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let mut configured = (*config()).clone();
    configured.compression_threshold = Some(1);
    let mut client = TestClient::connect(address).await;
    let (_remote, handle, driver) = listener
        .accept(Arc::new(configured), ConnectionLimits::default())
        .await
        .unwrap();
    let driver = tokio::spawn(driver.run());
    client
        .send(encode_handshake("localhost", address.port(), ClientIntent::Login).unwrap())
        .await;
    client
        .send(encode_login_hello("bbb-client", Uuid::nil()).unwrap())
        .await;

    let (compression_id, compression_payload) = client.read().await;
    assert_eq!(
        decode_login_clientbound(compression_id, &compression_payload).unwrap(),
        LoginClientbound::SetCompression { threshold: 1 }
    );
    client.compression_threshold = Some(1);
    let (finished_id, finished_payload) = client.read().await;
    assert!(matches!(
        decode_login_clientbound(finished_id, &finished_payload).unwrap(),
        LoginClientbound::LoginFinished { .. }
    ));

    handle.send(ConnectionCommand::Close).await.unwrap();
    let outcome = driver.await.unwrap().unwrap();
    assert_eq!(outcome.reason, ConnectionCloseReason::ServerCommand);
}

#[tokio::test]
async fn invalid_phase_order_closes_attributably() {
    let listener = ServerListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let mut client = TestClient::connect(address).await;
    let (_remote, _handle, driver) = listener
        .accept(config(), ConnectionLimits::default())
        .await
        .unwrap();
    let driver = tokio::spawn(driver.run());
    client.send(encode_status_request()).await;
    let outcome = driver.await.unwrap().unwrap();
    assert_eq!(outcome.final_phase, ConnectionPhase::Handshake);
    assert!(matches!(
        outcome.reason,
        ConnectionCloseReason::ProtocolViolation(reason)
            if reason.contains("buffer ended") || reason.contains("unknown")
    ));
}

#[test]
fn server_net_dependency_boundary_excludes_client_and_presentation_crates() {
    let manifest = include_str!("../Cargo.toml");
    for forbidden in [
        "bbb-client-net",
        "bbb-client-world",
        "bbb-client-pack",
        "bbb-client-renderer",
        "bbb-client-audio",
        "winit",
        "wgpu",
    ] {
        assert!(
            !manifest.lines().any(|line| {
                let line = line.trim_start();
                line.starts_with(forbidden)
                    && line[forbidden.len()..]
                        .starts_with(|ch: char| ch.is_whitespace() || ch == '=')
            }),
            "bbb-server-net must not depend on {forbidden}"
        );
    }
}

/// Handshake, hello and acknowledgement, then the client's information,
/// returning once the server's own opening packets are read.
async fn enter_configuration(
    config: ServerConnectionConfig,
    language: &str,
) -> (
    TestClient,
    crate::ConnectionHandle,
    tokio::task::JoinHandle<crate::Result<crate::ConnectionOutcome>>,
    Vec<ConfigurationClientbound>,
) {
    let listener = ServerListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let mut client = TestClient::connect(address).await;
    let (_remote, handle, driver) = listener
        .accept(Arc::new(config), ConnectionLimits::default())
        .await
        .unwrap();
    let driver = tokio::spawn(driver.run());
    client
        .send(encode_handshake("localhost", address.port(), ClientIntent::Login).unwrap())
        .await;
    client
        .send(encode_login_hello("bbb-client", Uuid::nil()).unwrap())
        .await;
    client.read().await;
    client.send(encode_login_acknowledged()).await;
    client
        .send(
            encode_client_information(&bbb_protocol::packets::ClientInformation {
                language: language.to_string(),
                ..Default::default()
            })
            .unwrap(),
        )
        .await;
    let mut opening = Vec::new();
    loop {
        let (id, payload) = client.read().await;
        let packet = decode_configuration_clientbound(id, &payload).unwrap();
        let done = matches!(packet, ConfigurationClientbound::SelectKnownPacks { .. });
        opening.push(packet);
        if done {
            break;
        }
    }
    (client, handle, driver, opening)
}

/// `ServerConfigurationPacketListenerImpl.startConfiguration` and
/// `addOptionalTasks` (`:83-128`): the bug report link comes right after the
/// brand, then after the registries the code of conduct in the client's
/// language, then the resource pack, and the finish only once each was
/// answered.
#[tokio::test]
async fn configuration_runs_the_link_conduct_and_pack_tasks_in_vanilla_order() {
    let mut configured = (*config()).clone();
    configured.bug_report_link = Some("https://example.invalid/bugs".to_string());
    configured.code_of_conduct = Arc::new(
        [
            ("en_us".to_string(), "Be kind.".to_string()),
            ("fr_fr".to_string(), "Soyez gentils.".to_string()),
        ]
        .into(),
    );
    let pack = Uuid::from_u128(5);
    configured.resource_pack = Some(crate::ServerResourcePack {
        id: pack,
        url: "https://example.invalid/pack.zip".to_string(),
        hash: String::new(),
        required: false,
        prompt: None,
    });
    let (mut client, handle, driver, opening) = enter_configuration(configured, "FR_FR").await;
    assert!(matches!(
        opening[0],
        ConfigurationClientbound::CustomPayload(_)
    ));
    assert!(matches!(
        &opening[1],
        ConfigurationClientbound::ServerLinks(links)
            if links.links.len() == 1 && links.links[0].url == "https://example.invalid/bugs"
    ));
    client.send(encode_select_known_packs(&[]).unwrap()).await;
    let (id, payload) = client.read().await;
    assert!(matches!(
        decode_configuration_clientbound(id, &payload).unwrap(),
        ConfigurationClientbound::UpdateTags(_)
    ));
    let (id, payload) = client.read().await;
    assert_eq!(
        decode_configuration_clientbound(id, &payload).unwrap(),
        ConfigurationClientbound::CodeOfConduct {
            text: "Soyez gentils.".to_string()
        }
    );
    client
        .send(bbb_protocol::packets::encode_configuration_accept_code_of_conduct())
        .await;
    let (id, payload) = client.read().await;
    assert!(matches!(
        decode_configuration_clientbound(id, &payload).unwrap(),
        ConfigurationClientbound::ResourcePackPush(push) if push.id == pack && !push.required
    ));
    for action in [
        ResourcePackResponseAction::Accepted,
        ResourcePackResponseAction::Downloaded,
        ResourcePackResponseAction::SuccessfullyLoaded,
    ] {
        client
            .send(bbb_protocol::packets::encode_configuration_resource_pack_response(pack, action))
            .await;
    }
    let (id, payload) = client.read().await;
    assert_eq!(
        decode_configuration_clientbound(id, &payload).unwrap(),
        ConfigurationClientbound::Finish
    );
    handle.send(ConnectionCommand::Close).await.unwrap();
    driver.await.unwrap().unwrap();
}

/// `ServerCommonPacketListenerImpl.handleResourcePackResponse` (`:105-111`).
#[tokio::test]
async fn declining_a_required_pack_disconnects_with_the_vanilla_reason() {
    let mut configured = (*config()).clone();
    let pack = Uuid::from_u128(6);
    configured.resource_pack = Some(crate::ServerResourcePack {
        id: pack,
        url: "https://example.invalid/pack.zip".to_string(),
        hash: String::new(),
        required: true,
        prompt: None,
    });
    let (mut client, _handle, driver, _) = enter_configuration(configured, "en_us").await;
    client.send(encode_select_known_packs(&[]).unwrap()).await;
    loop {
        let (id, payload) = client.read().await;
        if matches!(
            decode_configuration_clientbound(id, &payload).unwrap(),
            ConfigurationClientbound::ResourcePackPush(_)
        ) {
            break;
        }
    }
    client
        .send(
            bbb_protocol::packets::encode_configuration_resource_pack_response(
                pack,
                ResourcePackResponseAction::Declined,
            ),
        )
        .await;
    let (id, payload) = client.read().await;
    let packet = decode_configuration_clientbound(id, &payload).unwrap();
    assert!(
        matches!(&packet, ConfigurationClientbound::Disconnect { reason, .. }
            if reason.contains("multiplayer.requiredTexturePrompt.disconnect")),
        "{packet:?}"
    );
    let outcome = driver.await.unwrap().unwrap();
    assert_eq!(outcome.reason, ConnectionCloseReason::ResourcePackDeclined);
}

/// `ServerLoginPacketListenerImpl.verifyLoginAndFinishConnectionSetup`
/// (`:140-145`): a refused profile is told so in the login phase, before any
/// configuration starts.
#[tokio::test]
async fn a_refused_login_ends_with_a_login_disconnect() {
    let listener = ServerListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let mut configured = (*config()).clone();
    configured.admission = Some(crate::LoginAdmission(Arc::new(|profile, remote| {
        assert!(remote.is_some_and(|remote| remote.ip().is_loopback()));
        (profile.name == "banned").then(|| {
            bbb_protocol::RichTextComponent::try_from_component_nbt(
                bbb_protocol::component::component_translatable_nbt(
                    "multiplayer.disconnect.not_whitelisted",
                    Vec::new(),
                    &Default::default(),
                )
                .unwrap(),
            )
            .unwrap()
        })
    })));
    let mut client = TestClient::connect(address).await;
    let (_remote, _handle, driver) = listener
        .accept(Arc::new(configured), ConnectionLimits::default())
        .await
        .unwrap();
    let driver = tokio::spawn(driver.run());
    client
        .send(encode_handshake("localhost", address.port(), ClientIntent::Login).unwrap())
        .await;
    client
        .send(encode_login_hello("banned", Uuid::nil()).unwrap())
        .await;
    let (id, payload) = client.read().await;
    let packet = decode_login_clientbound(id, &payload).unwrap();
    assert!(
        matches!(&packet, LoginClientbound::Disconnect { raw_reason, .. }
            if raw_reason.contains("multiplayer.disconnect.not_whitelisted")),
        "{packet:?}"
    );
    let outcome = driver.await.unwrap().unwrap();
    assert_eq!(outcome.final_phase, ConnectionPhase::Login);
    assert_eq!(outcome.reason, ConnectionCloseReason::LoginRefused);
}

#[tokio::test]
async fn configuration_keep_alive_polls_and_disconnects_when_unanswered() {
    let listener = ServerListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let mut client = TestClient::connect(address).await;
    let (_remote, _handle, driver) = listener
        .accept(config(), ConnectionLimits::default())
        .await
        .unwrap();
    let driver = tokio::spawn(
        driver
            .with_keep_alive_interval(std::time::Duration::from_millis(40))
            .run(),
    );

    client
        .send(encode_handshake("localhost", address.port(), ClientIntent::Login).unwrap())
        .await;
    client
        .send(encode_login_hello("bbb-client", Uuid::nil()).unwrap())
        .await;
    let _ = client.read().await;
    client.send(encode_login_acknowledged()).await;
    // Drain brand, enabled_features, select_known_packs.
    for _ in 0..3 {
        let _ = client.read().await;
    }

    let (id, payload) = client.read().await;
    let ConfigurationClientbound::KeepAlive { id: challenge } =
        decode_configuration_clientbound(id, &payload).unwrap()
    else {
        panic!("expected configuration keep-alive challenge");
    };
    client
        .send(bbb_protocol::packets::encode_configuration_keep_alive(
            challenge,
        ))
        .await;

    let (id, payload) = client.read().await;
    assert!(matches!(
        decode_configuration_clientbound(id, &payload).unwrap(),
        ConfigurationClientbound::KeepAlive { .. }
    ));

    let (id, payload) = client.read().await;
    let packet = decode_configuration_clientbound(id, &payload).unwrap();
    assert!(
        matches!(
            &packet,
            ConfigurationClientbound::Disconnect { reason, .. }
                if reason.contains("disconnect.timeout")
        ),
        "{packet:?}"
    );
    let outcome = driver.await.unwrap().unwrap();
    assert_eq!(outcome.final_phase, ConnectionPhase::Configuration);
    assert_eq!(outcome.reason, ConnectionCloseReason::KeepAliveTimeout);
}

#[tokio::test]
async fn configuration_keep_alive_rejects_unsolicited_response() {
    let listener = ServerListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let mut client = TestClient::connect(address).await;
    let (_remote, _handle, driver) = listener
        .accept(config(), ConnectionLimits::default())
        .await
        .unwrap();
    let driver = tokio::spawn(driver.run());

    client
        .send(encode_handshake("localhost", address.port(), ClientIntent::Login).unwrap())
        .await;
    client
        .send(encode_login_hello("bbb-client", Uuid::nil()).unwrap())
        .await;
    let _ = client.read().await;
    client.send(encode_login_acknowledged()).await;
    for _ in 0..3 {
        let _ = client.read().await;
    }

    client
        .send(bbb_protocol::packets::encode_configuration_keep_alive(42))
        .await;
    let (id, payload) = client.read().await;
    let packet = decode_configuration_clientbound(id, &payload).unwrap();
    assert!(
        matches!(
            &packet,
            ConfigurationClientbound::Disconnect { reason, .. }
                if reason.contains("disconnect.timeout")
        ),
        "{packet:?}"
    );
    let outcome = driver.await.unwrap().unwrap();
    assert_eq!(outcome.final_phase, ConnectionPhase::Configuration);
    assert_eq!(outcome.reason, ConnectionCloseReason::KeepAliveTimeout);
}

#[tokio::test]
async fn configuration_accepts_custom_click_action_and_rejects_unsolicited_cookie_response() {
    let listener = ServerListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let mut client = TestClient::connect(address).await;
    let (_remote, _handle, driver) = listener
        .accept(config(), ConnectionLimits::default())
        .await
        .unwrap();
    let driver = tokio::spawn(driver.run());

    client
        .send(encode_handshake("localhost", address.port(), ClientIntent::Login).unwrap())
        .await;
    client
        .send(encode_login_hello("bbb-client", Uuid::nil()).unwrap())
        .await;
    let _ = client.read().await;
    client.send(encode_login_acknowledged()).await;
    for _ in 0..3 {
        let _ = client.read().await;
    }

    client
        .send(
            bbb_protocol::packets::encode_configuration_custom_click_action(
                &bbb_protocol::packets::CustomClickAction {
                    id: "bbb:dialog_ok".to_string(),
                    payload: None,
                },
            )
            .unwrap(),
        )
        .await;

    client
        .send(
            bbb_protocol::packets::encode_configuration_cookie_response("bbb:cookie", None)
                .unwrap(),
        )
        .await;
    let (id, payload) = client.read().await;
    let packet = decode_configuration_clientbound(id, &payload).unwrap();
    assert!(
        matches!(
            &packet,
            ConfigurationClientbound::Disconnect { reason, .. }
                if reason.contains("multiplayer.disconnect.unexpected_query_response")
        ),
        "{packet:?}"
    );
    let outcome = driver.await.unwrap().unwrap();
    assert_eq!(outcome.final_phase, ConnectionPhase::Configuration);
    assert_eq!(outcome.reason, ConnectionCloseReason::UnexpectedQuery);
}

#[tokio::test]
async fn login_rejects_unsolicited_query_or_cookie_with_unexpected_query_disconnect() {
    let listener = ServerListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let mut client = TestClient::connect(address).await;
    let (_remote, _handle, driver) = listener
        .accept(config(), ConnectionLimits::default())
        .await
        .unwrap();
    let driver = tokio::spawn(driver.run());

    client
        .send(encode_handshake("localhost", address.port(), ClientIntent::Login).unwrap())
        .await;
    client
        .send(bbb_protocol::packets::encode_login_custom_query_answer(5))
        .await;
    let (id, payload) = client.read().await;
    let packet = decode_login_clientbound(id, &payload).unwrap();
    assert!(
        matches!(
            &packet,
            LoginClientbound::Disconnect { raw_reason, .. }
                if raw_reason.contains("multiplayer.disconnect.unexpected_query_response")
        ),
        "{packet:?}"
    );
    let outcome = driver.await.unwrap().unwrap();
    assert_eq!(outcome.final_phase, ConnectionPhase::Login);
    assert_eq!(outcome.reason, ConnectionCloseReason::UnexpectedQuery);
}

#[tokio::test]
async fn login_rejects_encryption_key_packet_for_offline_mode() {
    let listener = ServerListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let mut client = TestClient::connect(address).await;
    let (_remote, _handle, driver) = listener
        .accept(config(), ConnectionLimits::default())
        .await
        .unwrap();
    let driver = tokio::spawn(driver.run());

    client
        .send(encode_handshake("localhost", address.port(), ClientIntent::Login).unwrap())
        .await;
    client
        .send(
            bbb_protocol::packets::encode_login_key(&bbb_protocol::packets::LoginKey {
                encrypted_secret: vec![1, 2],
                encrypted_challenge: vec![3, 4],
            })
            .unwrap(),
        )
        .await;
    let outcome = driver.await.unwrap().unwrap();
    assert_eq!(outcome.final_phase, ConnectionPhase::Login);
    assert!(matches!(
        outcome.reason,
        ConnectionCloseReason::ProtocolViolation(reason)
            if reason.contains("encryption key packet is invalid for offline login")
    ));
}

fn network_nbt_marker(marker: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(9 + marker.len());
    bytes.push(0x0a);
    bytes.push(0x08);
    bytes.extend_from_slice(&(6_u16).to_be_bytes());
    bytes.extend_from_slice(b"marker");
    bytes.extend_from_slice(&(marker.len() as u16).to_be_bytes());
    bytes.extend_from_slice(marker.as_bytes());
    bytes.push(0x00);
    bytes
}

#[tokio::test]
async fn select_known_packs_core_26_1_deduplicates_unmodified_builtin_entries_and_preserves_overrides(
) {
    let overworld_core = network_nbt_marker("overworld-core");
    let overworld_datapack = network_nbt_marker("overworld-datapack-override");
    let nether_core = network_nbt_marker("nether-core");
    let plains_core = network_nbt_marker("plains-core");
    let custom_mc_biome = network_nbt_marker("custom-mc-biome");
    let custom_ns_biome = network_nbt_marker("custom-ns-biome");

    let core_registries: Arc<[bbb_protocol::packets::PublishedRegistry]> = Arc::from(vec![
        bbb_protocol::packets::PublishedRegistry {
            registry: "minecraft:dimension_type".to_string(),
            entries: vec![
                bbb_protocol::packets::RegistryDataEntry {
                    id: "minecraft:overworld".to_string(),
                    raw_data: Some(overworld_core),
                },
                bbb_protocol::packets::RegistryDataEntry {
                    id: "minecraft:the_nether".to_string(),
                    raw_data: Some(nether_core.clone()),
                },
            ],
        },
        bbb_protocol::packets::PublishedRegistry {
            registry: "minecraft:worldgen/biome".to_string(),
            entries: vec![bbb_protocol::packets::RegistryDataEntry {
                id: "minecraft:plains".to_string(),
                raw_data: Some(plains_core.clone()),
            }],
        },
    ]);

    let published: Arc<[bbb_protocol::packets::PublishedRegistry]> = Arc::from(vec![
        bbb_protocol::packets::PublishedRegistry {
            registry: "minecraft:dimension_type".to_string(),
            entries: vec![
                bbb_protocol::packets::RegistryDataEntry {
                    id: "minecraft:overworld".to_string(),
                    raw_data: Some(overworld_datapack.clone()),
                },
                bbb_protocol::packets::RegistryDataEntry {
                    id: "minecraft:the_nether".to_string(),
                    raw_data: Some(nether_core.clone()),
                },
            ],
        },
        bbb_protocol::packets::PublishedRegistry {
            registry: "minecraft:worldgen/biome".to_string(),
            entries: vec![
                bbb_protocol::packets::RegistryDataEntry {
                    id: "example:aurora".to_string(),
                    raw_data: Some(custom_ns_biome.clone()),
                },
                bbb_protocol::packets::RegistryDataEntry {
                    id: "minecraft:custom_valley".to_string(),
                    raw_data: Some(custom_mc_biome.clone()),
                },
                bbb_protocol::packets::RegistryDataEntry {
                    id: "minecraft:plains".to_string(),
                    raw_data: Some(plains_core.clone()),
                },
            ],
        },
    ]);

    let mut configured = (*config()).clone();
    configured.known_packs = vec![crate::vanilla_core_known_pack()];
    configured.registries = Arc::clone(&published);
    configured.core_registries = Some(Arc::clone(&core_registries));

    // Confirm `minecraft:core` 26.1: unmodified core entries omit `raw_data`,
    // while overridden and custom entries preserve full NBT payloads and slot order.
    let (mut client, handle, driver, opening) =
        enter_configuration(configured.clone(), "en_us").await;
    assert_eq!(
        opening.last(),
        Some(&ConfigurationClientbound::SelectKnownPacks {
            known_packs: vec![crate::vanilla_core_known_pack()],
        })
    );
    client
        .send(encode_select_known_packs(&[crate::vanilla_core_known_pack()]).unwrap())
        .await;

    let (dim_id, dim_payload) = client.read().await;
    let ConfigurationClientbound::RegistryData(dim_data) =
        decode_configuration_clientbound(dim_id, &dim_payload).unwrap()
    else {
        panic!("expected dimension_type RegistryData");
    };
    assert_eq!(dim_data.registry, "minecraft:dimension_type");
    assert_eq!(
        dim_data.entries,
        vec![
            bbb_protocol::packets::RegistryDataEntry {
                id: "minecraft:overworld".to_string(),
                raw_data: Some(overworld_datapack.clone()),
            },
            bbb_protocol::packets::RegistryDataEntry {
                id: "minecraft:the_nether".to_string(),
                raw_data: None,
            },
        ]
    );

    let (biome_id, biome_payload) = client.read().await;
    let ConfigurationClientbound::RegistryData(biome_data) =
        decode_configuration_clientbound(biome_id, &biome_payload).unwrap()
    else {
        panic!("expected worldgen/biome RegistryData");
    };
    assert_eq!(biome_data.registry, "minecraft:worldgen/biome");
    assert_eq!(
        biome_data.entries,
        vec![
            bbb_protocol::packets::RegistryDataEntry {
                id: "example:aurora".to_string(),
                raw_data: Some(custom_ns_biome.clone()),
            },
            bbb_protocol::packets::RegistryDataEntry {
                id: "minecraft:custom_valley".to_string(),
                raw_data: Some(custom_mc_biome.clone()),
            },
            bbb_protocol::packets::RegistryDataEntry {
                id: "minecraft:plains".to_string(),
                raw_data: None,
            },
        ]
    );
    handle.send(ConnectionCommand::Close).await.unwrap();
    driver.await.unwrap().unwrap();

    // Declining known packs (or replying with another pack version) sends full NBT payloads.
    let (mut full_client, full_handle, full_driver, _) =
        enter_configuration(configured, "en_us").await;
    full_client
        .send(
            encode_select_known_packs(&[bbb_protocol::packets::KnownPack {
                namespace: "minecraft".to_string(),
                id: "core".to_string(),
                version: "1.21.4".to_string(),
            }])
            .unwrap(),
        )
        .await;
    let (full_dim_id, full_dim_payload) = full_client.read().await;
    let ConfigurationClientbound::RegistryData(full_dim_data) =
        decode_configuration_clientbound(full_dim_id, &full_dim_payload).unwrap()
    else {
        panic!("expected full dimension_type RegistryData");
    };
    assert_eq!(full_dim_data.entries[0].raw_data, Some(overworld_datapack));
    assert_eq!(full_dim_data.entries[1].raw_data, Some(nether_core));
    assert!(full_dim_payload.len() > dim_payload.len());

    let (full_biome_id, full_biome_payload) = full_client.read().await;
    let ConfigurationClientbound::RegistryData(full_biome_data) =
        decode_configuration_clientbound(full_biome_id, &full_biome_payload).unwrap()
    else {
        panic!("expected full worldgen/biome RegistryData");
    };
    assert_eq!(full_biome_data.entries[2].raw_data, Some(plains_core));
    assert!(full_biome_payload.len() > biome_payload.len());

    full_handle.send(ConnectionCommand::Close).await.unwrap();
    full_driver.await.unwrap().unwrap();
}

#[tokio::test]
async fn select_known_packs_core_26_1_uses_exact_builtin_catalog_when_core_baseline_is_absent() {
    let plains_nbt = network_nbt_marker("plains");
    let custom_mc_nbt = network_nbt_marker("custom-mc");
    let chat_nbt = network_nbt_marker("chat");

    let mut configured = (*config()).clone();
    configured.known_packs = vec![crate::vanilla_core_known_pack()];
    configured.registries = Arc::from(vec![
        bbb_protocol::packets::PublishedRegistry {
            registry: "minecraft:worldgen/biome".to_string(),
            entries: vec![
                bbb_protocol::packets::RegistryDataEntry {
                    id: "minecraft:custom_biome".to_string(),
                    raw_data: Some(custom_mc_nbt.clone()),
                },
                bbb_protocol::packets::RegistryDataEntry {
                    id: "minecraft:plains".to_string(),
                    raw_data: Some(plains_nbt),
                },
            ],
        },
        bbb_protocol::packets::PublishedRegistry {
            registry: "minecraft:chat_type".to_string(),
            entries: vec![bbb_protocol::packets::RegistryDataEntry {
                id: "minecraft:chat".to_string(),
                raw_data: Some(chat_nbt),
            }],
        },
    ]);

    let (mut client, handle, driver, _) = enter_configuration(configured, "en_us").await;
    client
        .send(encode_select_known_packs(&[crate::vanilla_core_known_pack()]).unwrap())
        .await;

    let (biome_id, biome_payload) = client.read().await;
    let ConfigurationClientbound::RegistryData(biome_data) =
        decode_configuration_clientbound(biome_id, &biome_payload).unwrap()
    else {
        panic!("expected worldgen/biome RegistryData");
    };
    assert_eq!(biome_data.entries[0].raw_data, Some(custom_mc_nbt));
    assert_eq!(biome_data.entries[1].raw_data, None);

    let (chat_id, chat_payload) = client.read().await;
    let ConfigurationClientbound::RegistryData(chat_data) =
        decode_configuration_clientbound(chat_id, &chat_payload).unwrap()
    else {
        panic!("expected chat_type RegistryData");
    };
    assert_eq!(chat_data.entries[0].raw_data, None);

    handle.send(ConnectionCommand::Close).await.unwrap();
    driver.await.unwrap().unwrap();
}

#[test]
fn rebuilt_connection_configs_share_preencoded_registry_sync_frames() {
    let registries: Arc<[bbb_protocol::packets::PublishedRegistry]> =
        Arc::from(vec![bbb_protocol::packets::PublishedRegistry {
            registry: "minecraft:dimension_type".to_string(),
            entries: vec![bbb_protocol::packets::RegistryDataEntry {
                id: "minecraft:overworld".to_string(),
                raw_data: Some(network_nbt_marker("overworld")),
            }],
        }]);
    let tags = Arc::new(bbb_protocol::packets::UpdateTags {
        registries: Vec::new(),
    });

    let mut first = ServerConnectionConfig::new(json!({"players": {"online": 0}}));
    first.registries = Arc::clone(&registries);
    first.tags = Arc::clone(&tags);

    let mut second = ServerConnectionConfig::new(json!({"players": {"online": 1}}));
    second.registries = Arc::clone(&registries);
    second.tags = Arc::clone(&tags);

    let first_ptr = crate::session::shared_registry_sync_ptr(&first).unwrap();
    let second_ptr = crate::session::shared_registry_sync_ptr(&second).unwrap();
    assert_eq!(first_ptr, second_ptr);
}

#[tokio::test]
async fn play_start_configuration_reenters_configuration_after_client_ack_and_returns_to_play() {
    let listener = ServerListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let mut client = TestClient::connect(address).await;
    let (_remote, handle, driver) = listener
        .accept(config(), ConnectionLimits::default())
        .await
        .unwrap();
    let driver = tokio::spawn(driver.run());

    client
        .send(encode_handshake("localhost", address.port(), ClientIntent::Login).unwrap())
        .await;
    client
        .send(encode_login_hello("reconfig-player", Uuid::from_u128(77)).unwrap())
        .await;
    let _ = client.read().await; // LoginFinished
    client.send(encode_login_acknowledged()).await;
    let _ = client.read().await; // Brand
    let _ = client.read().await; // EnabledFeatures
    let _ = client.read().await; // SelectKnownPacks
    client.send(encode_select_known_packs(&[]).unwrap()).await;
    let _ = client.read().await; // UpdateTags
    let _ = client.read().await; // FinishConfiguration
    client.send(encode_configuration_finish()).await;

    // Drain initial Login, Configuration, Play, PlayReady events.
    loop {
        let event = handle.next_event().await.unwrap();
        if matches!(event, ConnectionEvent::PlayReady { .. }) {
            break;
        }
    }

    // Request reconfiguration with an updated config offering `minecraft:core` 26.1.
    let mut updated = (*config()).clone();
    updated.known_packs = vec![crate::vanilla_core_known_pack()];
    handle
        .send(ConnectionCommand::StartConfiguration {
            config: Some(Arc::new(updated)),
        })
        .await
        .unwrap();
    // Stale outbound Play packets raced after StartConfiguration must be dropped cleanly.
    handle
        .send(ConnectionCommand::SendPlayPacket {
            packet_id: bbb_protocol::ids::play::CLIENTBOUND_BUNDLE_DELIMITER,
            payload: Arc::new(Vec::new()),
        })
        .await
        .unwrap();

    let (start_id, start_payload) = client.read().await;
    assert_eq!(
        start_id,
        bbb_protocol::ids::play::CLIENTBOUND_START_CONFIGURATION
    );
    assert!(start_payload.is_empty());

    // Client flushes one final Play packet before acknowledging configuration.
    client
        .send(bbb_protocol::packets::encode_play_pong(123))
        .await;
    assert_eq!(
        handle.next_event().await,
        Some(ConnectionEvent::PlayPacket {
            packet_id: bbb_protocol::ids::play::SERVERBOUND_PONG,
            payload: bbb_protocol::packets::encode_play_pong(123).1,
        })
    );

    client
        .send((
            bbb_protocol::ids::play::SERVERBOUND_CONFIGURATION_ACKNOWLEDGED,
            Vec::new(),
        ))
        .await;
    assert_eq!(
        handle.next_event().await,
        Some(ConnectionEvent::PhaseChanged {
            phase: ConnectionPhase::Configuration,
        })
    );

    let _ = client.read().await; // Brand
    let _ = client.read().await; // EnabledFeatures
    let (packs_id, packs_payload) = client.read().await;
    assert_eq!(
        decode_configuration_clientbound(packs_id, &packs_payload).unwrap(),
        ConfigurationClientbound::SelectKnownPacks {
            known_packs: vec![crate::vanilla_core_known_pack()],
        }
    );
    client
        .send(encode_select_known_packs(&[crate::vanilla_core_known_pack()]).unwrap())
        .await;
    let _ = client.read().await; // UpdateTags
    let (finish_id, finish_payload) = client.read().await;
    assert_eq!(
        decode_configuration_clientbound(finish_id, &finish_payload).unwrap(),
        ConfigurationClientbound::Finish
    );
    client.send(encode_configuration_finish()).await;

    assert_eq!(
        handle.next_event().await,
        Some(ConnectionEvent::PhaseChanged {
            phase: ConnectionPhase::Play,
        })
    );
    assert!(matches!(
        handle.next_event().await,
        Some(ConnectionEvent::PlayReady { profile, .. }) if profile.name == "reconfig-player"
    ));

    handle.send(ConnectionCommand::Close).await.unwrap();
    let outcome = driver.await.unwrap().unwrap();
    assert_eq!(outcome.final_phase, ConnectionPhase::Play);
    assert_eq!(outcome.reason, ConnectionCloseReason::ServerCommand);
}

#[tokio::test]
async fn handshake_and_status_enforce_read_idle_timeout() {
    let listener = ServerListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    // 1. Silent (or partial-frame) socket in Handshake closes at idle deadline.
    let mut handshake_client = TestClient::connect(address).await;
    let (_remote, _handle, driver) = listener
        .accept(config(), ConnectionLimits::default())
        .await
        .unwrap();
    let driver = tokio::spawn(
        driver
            .with_idle_timeout(std::time::Duration::from_millis(50))
            .run(),
    );
    handshake_client.stream.write_all(&[0x05]).await.unwrap();
    let outcome = tokio::time::timeout(std::time::Duration::from_secs(2), driver)
        .await
        .expect("handshake idle timeout")
        .unwrap()
        .unwrap();
    assert_eq!(outcome.final_phase, ConnectionPhase::Handshake);
    assert_eq!(outcome.reason, ConnectionCloseReason::IdleTimeout);

    // 2. Socket that enters Status and stops before Ping closes at idle deadline.
    let mut status_client = TestClient::connect(address).await;
    let (_remote, _handle, driver) = listener
        .accept(config(), ConnectionLimits::default())
        .await
        .unwrap();
    let driver = tokio::spawn(
        driver
            .with_idle_timeout(std::time::Duration::from_millis(50))
            .run(),
    );
    status_client
        .send(encode_handshake("localhost", address.port(), ClientIntent::Status).unwrap())
        .await;
    status_client.send(encode_status_request()).await;
    let (status_id, _) = status_client.read().await;
    assert_eq!(
        status_id,
        bbb_protocol::ids::status::CLIENTBOUND_STATUS_RESPONSE
    );
    let outcome = tokio::time::timeout(std::time::Duration::from_secs(2), driver)
        .await
        .expect("status idle timeout")
        .unwrap()
        .unwrap();
    assert_eq!(outcome.final_phase, ConnectionPhase::Status);
    assert_eq!(outcome.reason, ConnectionCloseReason::IdleTimeout);
}

#[tokio::test]
async fn serverbound_sub_threshold_compressed_frame_is_rejected() {
    let listener = ServerListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let mut configured = (*config()).clone();
    configured.compression_threshold = Some(256);
    let mut client = TestClient::connect(address).await;
    let (_remote, _handle, driver) = listener
        .accept(Arc::new(configured), ConnectionLimits::default())
        .await
        .unwrap();
    let driver = tokio::spawn(driver.run());

    client
        .send(encode_handshake("localhost", address.port(), ClientIntent::Login).unwrap())
        .await;
    client
        .send(encode_login_hello("bbb-client", Uuid::nil()).unwrap())
        .await;
    let (compression_id, compression_payload) = client.read().await;
    assert_eq!(
        decode_login_clientbound(compression_id, &compression_payload).unwrap(),
        LoginClientbound::SetCompression { threshold: 256 }
    );
    client.compression_threshold = Some(256);
    let _ = client.read().await; // LoginFinished
    client.send(encode_login_acknowledged()).await;

    // Craft a valid zlib-compressed frame whose uncompressed body length is below 256.
    let sub_threshold_frame = encode_packet_with_compression(
        bbb_protocol::ids::configuration::SERVERBOUND_KEEP_ALIVE,
        &42_i64.to_be_bytes(),
        Some(1),
    )
    .unwrap();
    client.stream.write_all(&sub_threshold_frame).await.unwrap();

    let outcome = tokio::time::timeout(std::time::Duration::from_secs(2), driver)
        .await
        .expect("driver closes on sub-threshold frame")
        .unwrap()
        .unwrap();
    assert_eq!(outcome.final_phase, ConnectionPhase::Configuration);
    assert!(matches!(
        outcome.reason,
        ConnectionCloseReason::ProtocolViolation(reason)
            if reason.contains("below compression threshold 256")
    ));
}

#[tokio::test]
async fn required_resource_pack_disconnects_on_all_terminal_failure_and_discard_statuses() {
    for action in [
        ResourcePackResponseAction::Declined,
        ResourcePackResponseAction::FailedDownload,
        ResourcePackResponseAction::InvalidUrl,
        ResourcePackResponseAction::FailedReload,
        ResourcePackResponseAction::Discarded,
    ] {
        let mut configured = (*config()).clone();
        let pack = Uuid::from_u128(6);
        configured.resource_pack = Some(crate::ServerResourcePack {
            id: pack,
            url: "https://example.invalid/pack.zip".to_string(),
            hash: String::new(),
            required: true,
            prompt: None,
        });
        let (mut client, _handle, driver, _) = enter_configuration(configured, "en_us").await;
        client.send(encode_select_known_packs(&[]).unwrap()).await;
        loop {
            let (id, payload) = client.read().await;
            if matches!(
                decode_configuration_clientbound(id, &payload).unwrap(),
                ConfigurationClientbound::ResourcePackPush(_)
            ) {
                break;
            }
        }
        client
            .send(bbb_protocol::packets::encode_configuration_resource_pack_response(pack, action))
            .await;
        let (id, payload) = client.read().await;
        let packet = decode_configuration_clientbound(id, &payload).unwrap();
        assert!(
            matches!(&packet, ConfigurationClientbound::Disconnect { reason, .. }
                if reason.contains("multiplayer.requiredTexturePrompt.disconnect")),
            "{action:?} -> {packet:?}"
        );
        let outcome = driver.await.unwrap().unwrap();
        assert_eq!(
            outcome.reason,
            ConnectionCloseReason::ResourcePackDeclined,
            "expected disconnect for {action:?}"
        );
    }
}

#[tokio::test]
async fn configuration_custom_click_action_emits_connection_event() {
    let (mut client, handle, driver, _) = enter_configuration((*config()).clone(), "en_us").await;
    let action = bbb_protocol::packets::CustomClickAction {
        id: "bbb:test_action".to_string(),
        payload: None,
    };
    client
        .send(bbb_protocol::packets::encode_configuration_custom_click_action(&action).unwrap())
        .await;
    let mut saw_action = None;
    for _ in 0..4 {
        match tokio::time::timeout(std::time::Duration::from_secs(2), handle.next_event())
            .await
            .expect("custom click event arrives")
        {
            Some(ConnectionEvent::CustomClickAction(received)) => {
                saw_action = Some(received);
                break;
            }
            Some(ConnectionEvent::PhaseChanged { .. }) => continue,
            other => panic!("unexpected event: {other:?}"),
        }
    }
    assert_eq!(saw_action, Some(action));
    handle.send(ConnectionCommand::Close).await.unwrap();
    driver.await.unwrap().unwrap();
}

#[tokio::test]
async fn serverbound_wire_fuzz_never_panics_driver_across_phases() {
    let listener = ServerListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let mut state = 0x517c_c1b7_2722_0a95_u64;
    let mut next_u64 = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    // Case 1: pure arbitrary bytes / oversized VarInts / truncated frames in handshake.
    for round in 0..16_usize {
        let client_fut = TcpStream::connect(address);
        let accept_fut = listener.accept(config(), ConnectionLimits::default());
        let (client_res, accept_res) = tokio::join!(client_fut, accept_fut);
        let mut stream = client_res.unwrap();
        let (_remote, handle, driver) = accept_res.unwrap();
        let driver = tokio::spawn(driver.run());

        let payload = match round % 4 {
            0 => vec![0xff; 12],
            1 => vec![0x80; 64],
            2 => encode_packet(round as i32, &vec![0xab; (round * 13) % 97]),
            _ => (0..((next_u64() % 128) + 1) as usize)
                .map(|_| next_u64() as u8)
                .collect(),
        };
        let _ = stream.write_all(&payload).await;
        drop(stream);
        drop(handle);
        let outcome = tokio::time::timeout(std::time::Duration::from_secs(2), driver)
            .await
            .expect("driver exits cleanly on malformed handshake")
            .expect("driver task does not panic")
            .unwrap();
        assert!(matches!(
            outcome.reason,
            ConnectionCloseReason::ProtocolViolation(_)
                | ConnectionCloseReason::RemoteClosed
                | ConnectionCloseReason::CommandProducerClosed
        ));
    }

    // Case 2: valid transition into Login/Configuration followed by fuzzed frames.
    for intent in [ClientIntent::Status, ClientIntent::Login] {
        for round in 0..8_usize {
            let client_fut = TestClient::connect(address);
            let accept_fut = listener.accept(config(), ConnectionLimits::default());
            let (mut client, accept_res) = tokio::join!(client_fut, accept_fut);
            let (_remote, handle, driver) = accept_res.unwrap();
            let driver = tokio::spawn(driver.run());

            client
                .send(encode_handshake("localhost", address.port(), intent).unwrap())
                .await;
            if matches!(intent, ClientIntent::Login) {
                client
                    .send(encode_login_hello("fuzz", Uuid::from_u128(1)).unwrap())
                    .await;
            }
            let fuzz_len = ((next_u64() % 128) + 1) as usize;
            let fuzz_body: Vec<u8> = (0..fuzz_len).map(|_| next_u64() as u8).collect();
            let packet_id = (round as i32) % 12;
            let frame = encode_packet(packet_id, &fuzz_body);
            let _ = client.stream.write_all(&frame).await;
            drop(client);
            drop(handle);
            let outcome = tokio::time::timeout(std::time::Duration::from_secs(2), driver)
                .await
                .expect("driver exits cleanly on fuzzed phase payload")
                .expect("driver task does not panic")
                .unwrap();
            assert!(matches!(
                outcome.reason,
                ConnectionCloseReason::ProtocolViolation(_)
                    | ConnectionCloseReason::UnexpectedQuery
                    | ConnectionCloseReason::RemoteClosed
                    | ConnectionCloseReason::CommandProducerClosed
                    | ConnectionCloseReason::StatusHandled
            ));
        }
    }
}
