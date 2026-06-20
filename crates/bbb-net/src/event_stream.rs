use std::collections::BTreeMap;

use anyhow::{anyhow, Context, Result};
use bbb_protocol::{
    frame::encode_packet,
    packets::{self, ClientIntent, PlayerPositionState},
};
use tokio::{
    sync::mpsc,
    time::{timeout, Interval},
};

use crate::{
    chunk_batch::ChunkBatchSizeCalculator,
    connection::RawConnection,
    driver::{read_packet_or_drive_connection, ConnectionDrive},
    types::{ConnectionOptions, ConnectionState, NetCommand, NetEvent},
};

mod configuration;
mod login;
mod play;

struct EventStreamContext {
    conn: RawConnection,
    events: mpsc::Sender<NetEvent>,
    commands: mpsc::Receiver<NetCommand>,
    state: ConnectionState,
    player_loaded_sent: bool,
    player_position_state: PlayerPositionState,
    play_tick: Option<Interval>,
    chunk_batch_size: ChunkBatchSizeCalculator,
    server_cookies: BTreeMap<String, Vec<u8>>,
    seen_code_of_conduct: bool,
    accepted_code_of_conduct_hash: Option<i32>,
    client_information: packets::ClientInformation,
}

pub async fn run_offline_event_stream(
    options: ConnectionOptions,
    events: mpsc::Sender<NetEvent>,
    commands: mpsc::Receiver<NetCommand>,
) -> Result<()> {
    let conn = timeout(
        options.timeout,
        RawConnection::connect(&options.address, None),
    )
    .await
    .context("offline connect timed out")??;
    let mut stream = EventStreamContext {
        conn,
        events,
        commands,
        state: ConnectionState::Login,
        player_loaded_sent: false,
        player_position_state: PlayerPositionState::default(),
        play_tick: None,
        chunk_batch_size: ChunkBatchSizeCalculator::new(),
        server_cookies: BTreeMap::new(),
        seen_code_of_conduct: false,
        accepted_code_of_conduct_hash: options.accepted_code_of_conduct_hash,
        client_information: options.client_information.clone(),
    };

    emit(&stream.events, NetEvent::Connected).await?;
    emit(
        &stream.events,
        NetEvent::StateChanged {
            state: stream.state,
        },
    )
    .await?;

    let (id, payload) = packets::encode_handshake(&options.host, options.port, ClientIntent::Login);
    stream.conn.send_packet(id, &payload).await?;
    let (id, payload) = packets::encode_login_hello(&options.username, options.profile_id);
    stream.conn.send_packet(id, &payload).await?;

    loop {
        let drive = read_packet_or_drive_connection(
            &mut stream.conn,
            stream.state,
            &mut stream.play_tick,
            &mut stream.commands,
            &mut stream.player_position_state,
        )
        .await?;
        let ConnectionDrive::Packet(packet_id, payload) = drive else {
            return Ok(());
        };
        tracing::debug!(
            state = ?stream.state,
            packet_id,
            len = payload.len(),
            "clientbound packet"
        );
        emit_best_effort(
            &stream.events,
            NetEvent::PacketSeen {
                state: stream.state,
                packet_id,
                len: payload.len(),
            },
        )?;

        match stream.state {
            ConnectionState::Login => {
                let packet = packets::decode_login_clientbound(packet_id, &payload)?;
                stream.handle_login_packet(packet).await?;
            }
            ConnectionState::Configuration => {
                let packet = packets::decode_configuration_clientbound(packet_id, &payload)?;
                stream.handle_configuration_packet(packet).await?;
            }
            ConnectionState::Play => {
                let packet = packets::decode_play_clientbound(packet_id, &payload)?;
                stream.handle_play_packet(packet).await?;
            }
            ConnectionState::Handshake | ConnectionState::Status => {
                unreachable!("event stream starts at login")
            }
        }
    }
}

pub(super) async fn emit(events: &mpsc::Sender<NetEvent>, event: NetEvent) -> Result<()> {
    events
        .send(event)
        .await
        .map_err(|_| anyhow!("net event receiver dropped"))
}

fn emit_best_effort(events: &mpsc::Sender<NetEvent>, event: NetEvent) -> Result<()> {
    if events.capacity() <= 1024 {
        return Ok(());
    }

    match events.try_send(event) {
        Ok(()) | Err(mpsc::error::TrySendError::Full(_)) => Ok(()),
        Err(mpsc::error::TrySendError::Closed(_)) => Err(anyhow!("net event receiver dropped")),
    }
}

#[allow(dead_code)]
fn _keep_encode_packet_reachable(packet_id: i32, payload: &[u8]) -> Vec<u8> {
    encode_packet(packet_id, payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::{
        codec::Decoder,
        ids,
        packets::{ChatAcknowledgement, GameProfile, LoginClientbound, PlayClientbound},
    };
    use bbb_world::code_of_conduct_text_hash;
    use bytes::BytesMut;
    use std::time::Duration;
    use tokio::{net::TcpListener, time::timeout};

    #[tokio::test]
    async fn login_finished_sends_brand_before_client_information() {
        let (client, mut server) = raw_connection_pair().await;
        let (events_tx, mut events_rx) = mpsc::channel(1);
        let (_commands_tx, commands_rx) = mpsc::channel(1);
        let mut stream = EventStreamContext {
            conn: client,
            events: events_tx,
            commands: commands_rx,
            state: ConnectionState::Login,
            player_loaded_sent: false,
            player_position_state: PlayerPositionState::default(),
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: true,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation {
                language: "zh_cn".to_string(),
                view_distance: 12,
                chat_visibility: packets::ClientChatVisibility::System,
                chat_colors: false,
                displayed_skin_parts: 0x15,
                main_hand: packets::ClientMainHand::Left,
                text_filtering_enabled: true,
                allows_listing: true,
                particle_status: packets::ClientParticleStatus::Minimal,
            },
        };

        stream
            .handle_login_packet(LoginClientbound::LoginFinished {
                profile: GameProfile {
                    uuid: uuid::Uuid::from_u128(0x12345678_1234_5678_90ab_cdef12345678),
                    name: "bbb-client".to_string(),
                    properties: Vec::new(),
                },
            })
            .await
            .unwrap();

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("login acknowledgement should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::login::SERVERBOUND_LOGIN_ACKNOWLEDGED);
        assert!(payload.is_empty());

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("brand custom payload should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::configuration::SERVERBOUND_CUSTOM_PAYLOAD);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_string(32767).unwrap(), "minecraft:brand");
        assert_eq!(decoder.read_string(32767).unwrap(), "bbb-native");
        assert!(decoder.is_empty());

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("client information should be sent")
            .unwrap();
        assert_eq!(
            packet_id,
            ids::configuration::SERVERBOUND_CLIENT_INFORMATION
        );
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_string(16).unwrap(), "zh_cn");
        assert_eq!(decoder.read_i8().unwrap(), 12);
        assert_eq!(decoder.read_var_i32().unwrap(), 1);
        assert!(!decoder.read_bool().unwrap());
        assert_eq!(decoder.read_u8().unwrap(), 0x15);
        assert_eq!(decoder.read_var_i32().unwrap(), 0);
        assert!(decoder.read_bool().unwrap());
        assert!(decoder.read_bool().unwrap());
        assert_eq!(decoder.read_var_i32().unwrap(), 2);
        assert!(decoder.is_empty());

        assert_eq!(stream.state, ConnectionState::Configuration);
        assert!(!stream.seen_code_of_conduct);
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("state-changed event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::StateChanged {
                state: ConnectionState::Configuration
            }
        ));
    }

    #[tokio::test]
    async fn configuration_finish_sends_configured_play_client_information() {
        let (client, mut server) = raw_connection_pair().await;
        let (events_tx, mut events_rx) = mpsc::channel(1);
        let (_commands_tx, commands_rx) = mpsc::channel(1);
        let mut stream = EventStreamContext {
            conn: client,
            events: events_tx,
            commands: commands_rx,
            state: ConnectionState::Configuration,
            player_loaded_sent: false,
            player_position_state: PlayerPositionState::default(),
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation {
                language: "ja_jp".to_string(),
                view_distance: 8,
                chat_visibility: packets::ClientChatVisibility::Hidden,
                chat_colors: true,
                displayed_skin_parts: 0x03,
                main_hand: packets::ClientMainHand::Right,
                text_filtering_enabled: false,
                allows_listing: true,
                particle_status: packets::ClientParticleStatus::Decreased,
            },
        };

        stream
            .handle_configuration_packet(packets::ConfigurationClientbound::Finish)
            .await
            .unwrap();

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("finish configuration should be sent")
            .unwrap();
        assert_eq!(
            packet_id,
            ids::configuration::SERVERBOUND_FINISH_CONFIGURATION
        );
        assert!(payload.is_empty());

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("play client information should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_CLIENT_INFORMATION);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_string(16).unwrap(), "ja_jp");
        assert_eq!(decoder.read_i8().unwrap(), 8);
        assert_eq!(decoder.read_var_i32().unwrap(), 2);
        assert!(decoder.read_bool().unwrap());
        assert_eq!(decoder.read_u8().unwrap(), 0x03);
        assert_eq!(decoder.read_var_i32().unwrap(), 1);
        assert!(!decoder.read_bool().unwrap());
        assert!(decoder.read_bool().unwrap());
        assert_eq!(decoder.read_var_i32().unwrap(), 1);
        assert!(decoder.is_empty());
        assert_eq!(stream.state, ConnectionState::Play);
        assert!(stream.play_tick.is_some());

        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("state-changed event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::StateChanged {
                state: ConnectionState::Play
            }
        ));
    }

    #[tokio::test]
    async fn configuration_code_of_conduct_emits_event_without_immediate_accept() {
        let (client, mut server) = raw_connection_pair().await;
        let (events_tx, mut events_rx) = mpsc::channel(4);
        let (_commands_tx, commands_rx) = mpsc::channel(1);
        let mut stream = EventStreamContext {
            conn: client,
            events: events_tx,
            commands: commands_rx,
            state: ConnectionState::Configuration,
            player_loaded_sent: false,
            player_position_state: PlayerPositionState::default(),
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
        };

        stream
            .handle_configuration_packet(packets::ConfigurationClientbound::CodeOfConduct {
                text: "Follow the server rules.".to_string(),
            })
            .await
            .unwrap();

        assert!(timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err());

        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("code-of-conduct event should be emitted")
            .unwrap();
        match event {
            NetEvent::CodeOfConduct { text } => {
                assert_eq!(text, "Follow the server rules.");
            }
            other => panic!("expected code-of-conduct event, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn configuration_code_of_conduct_auto_accepts_matching_hash() {
        let (client, mut server) = raw_connection_pair().await;
        let (events_tx, mut events_rx) = mpsc::channel(4);
        let (_commands_tx, commands_rx) = mpsc::channel(1);
        let text = "Follow the server rules.";
        let mut stream = EventStreamContext {
            conn: client,
            events: events_tx,
            commands: commands_rx,
            state: ConnectionState::Configuration,
            player_loaded_sent: false,
            player_position_state: PlayerPositionState::default(),
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: Some(code_of_conduct_text_hash(text)),
            client_information: packets::ClientInformation::default(),
        };

        stream
            .handle_configuration_packet(packets::ConfigurationClientbound::CodeOfConduct {
                text: text.to_string(),
            })
            .await
            .unwrap();

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("accept packet should be sent")
            .unwrap();
        assert_eq!(
            packet_id,
            ids::configuration::SERVERBOUND_ACCEPT_CODE_OF_CONDUCT
        );
        assert!(payload.is_empty());

        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("code-of-conduct event should be emitted")
            .unwrap();
        assert!(
            matches!(event, NetEvent::CodeOfConduct { text } if text == "Follow the server rules.")
        );
    }

    #[tokio::test]
    async fn configuration_code_of_conduct_rejects_duplicate_packet() {
        let (client, mut server) = raw_connection_pair().await;
        let (events_tx, mut events_rx) = mpsc::channel(4);
        let (_commands_tx, commands_rx) = mpsc::channel(1);
        let mut stream = EventStreamContext {
            conn: client,
            events: events_tx,
            commands: commands_rx,
            state: ConnectionState::Configuration,
            player_loaded_sent: false,
            player_position_state: PlayerPositionState::default(),
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
        };

        stream
            .handle_configuration_packet(packets::ConfigurationClientbound::CodeOfConduct {
                text: "First rules.".to_string(),
            })
            .await
            .unwrap();
        timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("first code-of-conduct event should be emitted")
            .unwrap();

        let err = stream
            .handle_configuration_packet(packets::ConfigurationClientbound::CodeOfConduct {
                text: "Second rules.".to_string(),
            })
            .await
            .unwrap_err();

        assert!(
            err.to_string().contains("duplicate Code of Conduct"),
            "{err:?}"
        );
        assert!(matches!(
            events_rx.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
        assert!(timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn configuration_resource_pack_push_emits_push_and_response_events() {
        let (client, mut server) = raw_connection_pair().await;
        let (events_tx, mut events_rx) = mpsc::channel(2);
        let (_commands_tx, commands_rx) = mpsc::channel(1);
        let mut stream = EventStreamContext {
            conn: client,
            events: events_tx,
            commands: commands_rx,
            state: ConnectionState::Configuration,
            player_loaded_sent: false,
            player_position_state: PlayerPositionState::default(),
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
        };
        let pack_id = uuid::Uuid::from_u128(0x12345678_1234_5678_90ab_cdef12345678);

        stream
            .handle_configuration_packet(packets::ConfigurationClientbound::ResourcePackPush(
                packets::ResourcePackPush {
                    id: pack_id,
                    url: "https://example.invalid/config-pack.zip".to_string(),
                    hash: "0123456789abcdef0123456789abcdef01234567".to_string(),
                    required: false,
                    prompt: None,
                },
            ))
            .await
            .unwrap();

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("resource pack response should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::configuration::SERVERBOUND_RESOURCE_PACK);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_uuid().unwrap(), pack_id);
        assert_eq!(
            decoder.read_var_i32().unwrap(),
            packets::ResourcePackResponseAction::Declined.ordinal()
        );
        assert!(decoder.is_empty());

        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("resource pack push event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::ResourcePackPush(update) if update.id == pack_id
        ));
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("resource pack response event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::ResourcePackResponse {
                id,
                action: packets::ResourcePackResponseAction::Declined
            } if id == pack_id
        ));
    }

    #[tokio::test]
    async fn configuration_resource_pack_push_with_invalid_url_sends_invalid_url_response() {
        let (client, mut server) = raw_connection_pair().await;
        let (events_tx, mut events_rx) = mpsc::channel(2);
        let (_commands_tx, commands_rx) = mpsc::channel(1);
        let mut stream = EventStreamContext {
            conn: client,
            events: events_tx,
            commands: commands_rx,
            state: ConnectionState::Configuration,
            player_loaded_sent: false,
            player_position_state: PlayerPositionState::default(),
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
        };
        let pack_id = uuid::Uuid::from_u128(0x33333333_1234_5678_90ab_cdef12345678);

        stream
            .handle_configuration_packet(packets::ConfigurationClientbound::ResourcePackPush(
                packets::ResourcePackPush {
                    id: pack_id,
                    url: "ftp://example.invalid/config-pack.zip".to_string(),
                    hash: "0123456789abcdef0123456789abcdef01234567".to_string(),
                    required: false,
                    prompt: None,
                },
            ))
            .await
            .unwrap();

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("resource pack response should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::configuration::SERVERBOUND_RESOURCE_PACK);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_uuid().unwrap(), pack_id);
        assert_eq!(
            decoder.read_var_i32().unwrap(),
            packets::ResourcePackResponseAction::InvalidUrl.ordinal()
        );
        assert!(decoder.is_empty());

        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("resource pack push event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::ResourcePackPush(update) if update.id == pack_id
        ));
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("resource pack response event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::ResourcePackResponse {
                id,
                action: packets::ResourcePackResponseAction::InvalidUrl
            } if id == pack_id
        ));
    }

    #[tokio::test]
    async fn configuration_select_known_packs_emits_event_and_sends_empty_response() {
        let (client, mut server) = raw_connection_pair().await;
        let (events_tx, mut events_rx) = mpsc::channel(4);
        let (_commands_tx, commands_rx) = mpsc::channel(1);
        let mut stream = EventStreamContext {
            conn: client,
            events: events_tx,
            commands: commands_rx,
            state: ConnectionState::Configuration,
            player_loaded_sent: false,
            player_position_state: PlayerPositionState::default(),
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
        };

        stream
            .handle_configuration_packet(packets::ConfigurationClientbound::SelectKnownPacks {
                known_packs: vec![packets::KnownPack {
                    namespace: "minecraft".to_string(),
                    id: "core".to_string(),
                    version: "26.1".to_string(),
                }],
            })
            .await
            .unwrap();

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("select-known-packs response should be sent")
            .unwrap();
        assert_eq!(
            packet_id,
            ids::configuration::SERVERBOUND_SELECT_KNOWN_PACKS
        );
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 0);
        assert!(decoder.is_empty());

        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("select-known-packs event should be emitted")
            .unwrap();
        match event {
            NetEvent::SelectKnownPacks {
                known_packs,
                selected_packs,
            } => {
                assert_eq!(known_packs.len(), 1);
                assert_eq!(known_packs[0].namespace, "minecraft");
                assert_eq!(known_packs[0].id, "core");
                assert_eq!(known_packs[0].version, "26.1");
                assert!(selected_packs.is_empty());
            }
            other => panic!("expected select-known-packs event, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn configuration_unknown_packets_emit_unsupported_packet_events() {
        let (client, mut server) = raw_connection_pair().await;
        let (events_tx, mut events_rx) = mpsc::channel(1);
        let (_commands_tx, commands_rx) = mpsc::channel(1);
        let mut stream = EventStreamContext {
            conn: client,
            events: events_tx,
            commands: commands_rx,
            state: ConnectionState::Configuration,
            player_loaded_sent: false,
            player_position_state: PlayerPositionState::default(),
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
        };

        stream
            .handle_configuration_packet(packets::ConfigurationClientbound::Unknown {
                packet_id: 0x7f,
                len: 12,
            })
            .await
            .unwrap();

        assert!(timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err());
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("unsupported packet event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::UnsupportedPacket {
                state: ConnectionState::Configuration,
                packet_id: 0x7f,
                len: 12,
            }
        ));
    }

    #[tokio::test]
    async fn login_unknown_packets_emit_unsupported_packet_events() {
        let (client, mut server) = raw_connection_pair().await;
        let (events_tx, mut events_rx) = mpsc::channel(1);
        let (_commands_tx, commands_rx) = mpsc::channel(1);
        let mut stream = EventStreamContext {
            conn: client,
            events: events_tx,
            commands: commands_rx,
            state: ConnectionState::Login,
            player_loaded_sent: false,
            player_position_state: PlayerPositionState::default(),
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
        };

        stream
            .handle_login_packet(LoginClientbound::Unknown {
                packet_id: 0x7d,
                len: 5,
            })
            .await
            .unwrap();

        assert!(timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err());
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("unsupported packet event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::UnsupportedPacket {
                state: ConnectionState::Login,
                packet_id: 0x7d,
                len: 5,
            }
        ));
    }

    #[tokio::test]
    async fn play_chunk_batch_feedback_uses_vanilla_calculator() {
        let (client, mut server) = raw_connection_pair().await;
        let (events_tx, mut events_rx) = mpsc::channel(1);
        let (_commands_tx, commands_rx) = mpsc::channel(1);
        let mut stream = EventStreamContext {
            conn: client,
            events: events_tx,
            commands: commands_rx,
            state: ConnectionState::Play,
            player_loaded_sent: false,
            player_position_state: PlayerPositionState::default(),
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
        };
        stream
            .handle_play_packet(PlayClientbound::ChunkBatchStart)
            .await
            .unwrap();
        assert!(
            timeout(Duration::from_millis(50), server.read_packet())
                .await
                .is_err(),
            "chunk batch start must not send serverbound packets"
        );
        assert!(
            timeout(Duration::from_millis(50), events_rx.recv())
                .await
                .is_err(),
            "chunk batch start must not emit gameplay events"
        );

        stream
            .handle_play_packet(PlayClientbound::ChunkBatchFinished { batch_size: 0 })
            .await
            .unwrap();

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("chunk batch received packet should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_CHUNK_BATCH_RECEIVED);
        let desired = Decoder::new(&payload).read_f32().unwrap();
        assert_eq!(desired, 3.5);
    }

    #[tokio::test]
    async fn play_keep_alive_and_ping_send_common_responses() {
        let (client, mut server) = raw_connection_pair().await;
        let (events_tx, _events_rx) = mpsc::channel(1);
        let (_commands_tx, commands_rx) = mpsc::channel(1);
        let mut stream = EventStreamContext {
            conn: client,
            events: events_tx,
            commands: commands_rx,
            state: ConnectionState::Play,
            player_loaded_sent: false,
            player_position_state: PlayerPositionState::default(),
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
        };

        stream
            .handle_play_packet(PlayClientbound::KeepAlive {
                id: 0x1122_3344_5566_7788,
            })
            .await
            .unwrap();
        stream
            .handle_play_packet(PlayClientbound::Ping { id: 0x0a0b_0c0d })
            .await
            .unwrap();

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("keep alive response should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_KEEP_ALIVE);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_i64().unwrap(), 0x1122_3344_5566_7788);
        assert!(decoder.is_empty());

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("pong response should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_PONG);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_i32().unwrap(), 0x0a0b_0c0d);
        assert!(decoder.is_empty());
    }

    #[tokio::test]
    async fn play_entity_and_respawn_packets_emit_matching_events() {
        let (client, mut server) = raw_connection_pair().await;
        let (events_tx, mut events_rx) = mpsc::channel(4);
        let (_commands_tx, commands_rx) = mpsc::channel(1);
        let mut stream = EventStreamContext {
            conn: client,
            events: events_tx,
            commands: commands_rx,
            state: ConnectionState::Play,
            player_loaded_sent: false,
            player_position_state: PlayerPositionState::default(),
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
        };

        let block_destruction = packets::BlockDestruction {
            id: 4,
            pos: packets::BlockPos {
                x: 12,
                y: 64,
                z: -5,
            },
            progress: 6,
        };
        stream
            .handle_play_packet(PlayClientbound::BlockDestruction(block_destruction))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("block destruction event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::BlockDestruction(update) if update == block_destruction
        ));

        let entity_move = packets::EntityMove {
            id: 123,
            delta_x: 4096,
            delta_y: 0,
            delta_z: -2048,
            y_rot: Some(-90.0),
            x_rot: Some(45.0),
            on_ground: false,
        };
        stream
            .handle_play_packet(PlayClientbound::MoveEntity(entity_move))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("entity move event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::MoveEntity(update) if update == entity_move
        ));

        let entity_event = packets::EntityEvent {
            entity_id: 123,
            event_id: 35,
        };
        stream
            .handle_play_packet(PlayClientbound::EntityEvent(entity_event))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("entity event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::EntityEvent(update) if update == entity_event
        ));

        let respawn = packets::Respawn {
            common_spawn_info: packets::CommonPlayerSpawnInfo {
                dimension_type_id: 1,
                dimension: "minecraft:the_nether".to_string(),
                seed: 42,
                game_type: 1,
                previous_game_type: 0,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 20,
                sea_level: 32,
            },
            data_to_keep: 0,
        };
        stream
            .handle_play_packet(PlayClientbound::Respawn(respawn.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("respawn event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::Respawn(update) if update == respawn));

        assert!(
            timeout(Duration::from_millis(50), server.read_packet())
                .await
                .is_err(),
            "entity and respawn dispatcher packets must not send serverbound responses"
        );
    }

    #[tokio::test]
    async fn play_resource_pack_push_emits_push_and_response_events() {
        let (client, mut server) = raw_connection_pair().await;
        let (events_tx, mut events_rx) = mpsc::channel(2);
        let (_commands_tx, commands_rx) = mpsc::channel(1);
        let mut stream = EventStreamContext {
            conn: client,
            events: events_tx,
            commands: commands_rx,
            state: ConnectionState::Play,
            player_loaded_sent: false,
            player_position_state: PlayerPositionState::default(),
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
        };
        let pack_id = uuid::Uuid::from_u128(0x87654321_4321_8765_90ab_cdef12345678);

        stream
            .handle_play_packet(PlayClientbound::ResourcePackPush(
                packets::ResourcePackPush {
                    id: pack_id,
                    url: "https://example.invalid/play-pack.zip".to_string(),
                    hash: "abcdef0123456789abcdef0123456789abcdef01".to_string(),
                    required: true,
                    prompt: Some("Install pack?".to_string()),
                },
            ))
            .await
            .unwrap();

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("resource pack response should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_RESOURCE_PACK);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_uuid().unwrap(), pack_id);
        assert_eq!(
            decoder.read_var_i32().unwrap(),
            packets::ResourcePackResponseAction::Declined.ordinal()
        );
        assert!(decoder.is_empty());

        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("resource pack push event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::ResourcePackPush(update) if update.id == pack_id
        ));
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("resource pack response event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::ResourcePackResponse {
                id,
                action: packets::ResourcePackResponseAction::Declined
            } if id == pack_id
        ));
    }

    #[tokio::test]
    async fn play_resource_pack_push_with_invalid_url_sends_invalid_url_response() {
        let (client, mut server) = raw_connection_pair().await;
        let (events_tx, mut events_rx) = mpsc::channel(2);
        let (_commands_tx, commands_rx) = mpsc::channel(1);
        let mut stream = EventStreamContext {
            conn: client,
            events: events_tx,
            commands: commands_rx,
            state: ConnectionState::Play,
            player_loaded_sent: false,
            player_position_state: PlayerPositionState::default(),
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
        };
        let pack_id = uuid::Uuid::from_u128(0x44444444_4321_8765_90ab_cdef12345678);

        stream
            .handle_play_packet(PlayClientbound::ResourcePackPush(
                packets::ResourcePackPush {
                    id: pack_id,
                    url: "not a valid resource pack url".to_string(),
                    hash: "abcdef0123456789abcdef0123456789abcdef01".to_string(),
                    required: true,
                    prompt: Some("Install pack?".to_string()),
                },
            ))
            .await
            .unwrap();

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("resource pack response should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_RESOURCE_PACK);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_uuid().unwrap(), pack_id);
        assert_eq!(
            decoder.read_var_i32().unwrap(),
            packets::ResourcePackResponseAction::InvalidUrl.ordinal()
        );
        assert!(decoder.is_empty());

        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("resource pack push event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::ResourcePackPush(update) if update.id == pack_id
        ));
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("resource pack response event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::ResourcePackResponse {
                id,
                action: packets::ResourcePackResponseAction::InvalidUrl
            } if id == pack_id
        ));
    }

    #[tokio::test]
    async fn play_server_presentation_packets_emit_matching_events_and_cookie_response() {
        let (client, mut server) = raw_connection_pair().await;
        let (events_tx, mut events_rx) = mpsc::channel(8);
        let (_commands_tx, commands_rx) = mpsc::channel(1);
        let mut stream = EventStreamContext {
            conn: client,
            events: events_tx,
            commands: commands_rx,
            state: ConnectionState::Play,
            player_loaded_sent: false,
            player_position_state: PlayerPositionState::default(),
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
        };

        stream
            .handle_play_packet(PlayClientbound::StoreCookie(packets::StoreCookie {
                key: "bbb:session".to_string(),
                payload: vec![4, 5, 6],
            }))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("store cookie event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::StoreCookie {
                key,
                payload_len: 3,
                stored_cookie_count: 1,
            } if key == "bbb:session"
        ));

        stream
            .handle_play_packet(PlayClientbound::CookieRequest(packets::CookieRequest {
                key: "bbb:session".to_string(),
            }))
            .await
            .unwrap();
        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("cookie response should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_COOKIE_RESPONSE);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_string(32767).unwrap(), "bbb:session");
        assert!(decoder.read_bool().unwrap());
        let len = decoder.read_len().unwrap();
        assert_eq!(
            decoder.read_exact(len, "cookie response").unwrap(),
            &[4, 5, 6]
        );
        assert!(decoder.is_empty());
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("cookie request event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::CookieRequest {
                key,
                response_payload_present: true,
            } if key == "bbb:session"
        ));

        let custom_payload = packets::CustomPayload {
            id: "minecraft:brand".to_string(),
            payload: packets::CustomPayloadBody::Brand {
                brand: "vanilla".to_string(),
            },
        };
        stream
            .handle_play_packet(PlayClientbound::CustomPayload(custom_payload.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("custom payload event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::CustomPayload(update) if update == custom_payload));

        let mut details = BTreeMap::new();
        details.insert("Server".to_string(), "play".to_string());
        let custom_report_details = packets::CustomReportDetails { details };
        stream
            .handle_play_packet(PlayClientbound::CustomReportDetails(
                custom_report_details.clone(),
            ))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("custom report details event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::CustomReportDetails(update) if update == custom_report_details
        ));

        let server_links = packets::ServerLinks {
            links: vec![packets::ServerLinkEntry {
                link_type: packets::ServerLinkType::Known(packets::ServerLinkKnownType::Website),
                url: "https://example.invalid".to_string(),
            }],
        };
        stream
            .handle_play_packet(PlayClientbound::ServerLinks(server_links.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("server links event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::ServerLinks(update) if update == server_links));

        let server_data = packets::ServerData {
            motd: "Offline play server".to_string(),
            icon_bytes: Some(vec![0x89, b'P', b'N', b'G']),
        };
        stream
            .handle_play_packet(PlayClientbound::ServerData(server_data.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("server data event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::ServerData(update) if update == server_data));

        let resource_pack_pop = packets::ResourcePackPop {
            id: Some(uuid::Uuid::from_u128(
                0x11111111_2222_3333_4444_555555555555,
            )),
        };
        stream
            .handle_play_packet(PlayClientbound::ResourcePackPop(resource_pack_pop.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("resource pack pop event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::ResourcePackPop(update) if update == resource_pack_pop
        ));

        let transfer = packets::Transfer {
            host: "next.example.invalid".to_string(),
            port: 25566,
        };
        stream
            .handle_play_packet(PlayClientbound::Transfer(transfer.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("transfer event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::Transfer(update) if update == transfer));

        assert!(
            timeout(Duration::from_millis(50), server.read_packet())
                .await
                .is_err(),
            "server-presentation packets after cookie response must not send extra responses"
        );
    }

    #[tokio::test]
    async fn play_inventory_packets_emit_matching_events_without_responses() {
        let (client, mut server) = raw_connection_pair().await;
        let (events_tx, mut events_rx) = mpsc::channel(8);
        let (_commands_tx, commands_rx) = mpsc::channel(1);
        let mut stream = EventStreamContext {
            conn: client,
            events: events_tx,
            commands: commands_rx,
            state: ConnectionState::Play,
            player_loaded_sent: false,
            player_position_state: PlayerPositionState::default(),
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
        };

        let open_screen = packets::OpenScreen {
            container_id: 7,
            menu_type_id: 19,
            title: "Merchant".to_string(),
        };
        stream
            .handle_play_packet(PlayClientbound::OpenScreen(open_screen.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("open screen event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::OpenScreen(update) if update == open_screen));

        let content = packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![packets::ItemStackSummary::empty(), test_item_stack(42, 3)],
            carried_item: test_item_stack(99, 1),
        };
        stream
            .handle_play_packet(PlayClientbound::ContainerSetContent(content.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("container content event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::ContainerSetContent(update) if update == content
        ));

        let slot = packets::ContainerSetSlot {
            container_id: 7,
            state_id: 13,
            slot: 1,
            item: test_item_stack(43, 2),
        };
        stream
            .handle_play_packet(PlayClientbound::ContainerSetSlot(slot.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("container slot event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::ContainerSetSlot(update) if update == slot));

        let data = packets::ContainerSetData {
            container_id: 7,
            id: 2,
            value: 10,
        };
        stream
            .handle_play_packet(PlayClientbound::ContainerSetData(data))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("container data event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::ContainerSetData(update) if update == data));

        let player_slot = packets::SetPlayerInventory {
            slot: 36,
            item: test_item_stack(44, 1),
        };
        stream
            .handle_play_packet(PlayClientbound::SetPlayerInventory(player_slot.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("player inventory event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::SetPlayerInventory(update) if update == player_slot
        ));

        let cursor = packets::SetCursorItem {
            item: test_item_stack(45, 1),
        };
        stream
            .handle_play_packet(PlayClientbound::SetCursorItem(cursor.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("cursor item event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::SetCursorItem(update) if update == cursor));

        let offers = packets::MerchantOffers {
            container_id: 7,
            offers: vec![packets::MerchantOffer {
                buy_a: packets::ItemCostSummary {
                    item_id: 42,
                    count: 3,
                    component_predicate: packets::ItemCostComponentPredicateSummary::default(),
                },
                sell: test_item_stack(99, 1),
                buy_b: None,
                is_out_of_stock: false,
                uses: 1,
                max_uses: 8,
                xp: 5,
                special_price_diff: 0,
                price_multiplier: 0.05,
                demand: 2,
            }],
            villager_level: 3,
            villager_xp: 120,
            show_progress: true,
            can_restock: false,
        };
        stream
            .handle_play_packet(PlayClientbound::MerchantOffers(offers.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("merchant offers event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::MerchantOffers(update) if update == offers));

        let close = packets::ContainerClose { container_id: 7 };
        stream
            .handle_play_packet(PlayClientbound::ContainerClose(close))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("container close event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::ContainerClose(update) if update == close));

        assert!(
            timeout(Duration::from_millis(50), server.read_packet())
                .await
                .is_err(),
            "inventory update packets must not send serverbound responses"
        );
    }

    #[tokio::test]
    async fn play_command_and_chat_packets_emit_matching_events_without_responses() {
        let (client, mut server) = raw_connection_pair().await;
        let (events_tx, mut events_rx) = mpsc::channel(9);
        let (_commands_tx, commands_rx) = mpsc::channel(1);
        let mut stream = EventStreamContext {
            conn: client,
            events: events_tx,
            commands: commands_rx,
            state: ConnectionState::Play,
            player_loaded_sent: false,
            player_position_state: PlayerPositionState::default(),
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
        };

        let commands = command_tree_packet("say");
        stream
            .handle_play_packet(PlayClientbound::Commands(commands.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("commands event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::Commands(update) if update == commands));

        let completions = packets::CustomChatCompletions {
            action: packets::CustomChatCompletionsAction::Set,
            entries: vec!["/spawn".to_string(), "/warp".to_string()],
        };
        stream
            .handle_play_packet(PlayClientbound::CustomChatCompletions(completions.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("custom chat completions event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::CustomChatCompletions(update) if update == completions
        ));

        let suggestions = packets::CommandSuggestions {
            id: 77,
            start: 1,
            length: 4,
            suggestions: vec![
                packets::CommandSuggestion {
                    text: "give".to_string(),
                    tooltip: Some("Run give".to_string()),
                },
                packets::CommandSuggestion {
                    text: "gamemode".to_string(),
                    tooltip: None,
                },
            ],
        };
        stream
            .handle_play_packet(PlayClientbound::CommandSuggestions(suggestions.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("command suggestions event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::CommandSuggestions(update) if update == suggestions
        ));

        let tag_query = packets::TagQuery {
            transaction_id: 12,
            tag_present: true,
            raw_nbt: vec![10, 0],
        };
        stream
            .handle_play_packet(PlayClientbound::TagQuery(tag_query.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("tag query event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::TagQuery(update) if update == tag_query));

        let player_chat = player_chat_packet(3);
        stream
            .handle_play_packet(PlayClientbound::PlayerChat(player_chat.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("player chat event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::PlayerChat(update) if update == player_chat));

        let delete_chat = packets::DeleteChat {
            message_signature: packets::PackedMessageSignature {
                cache_id: Some(2),
                full_signature: None,
            },
        };
        stream
            .handle_play_packet(PlayClientbound::DeleteChat(delete_chat.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("delete chat event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::DeleteChat(update) if update == delete_chat));

        let disguised_chat = packets::DisguisedChat {
            message: "Server says hi".to_string(),
            chat_type: chat_type_bound("Server"),
        };
        stream
            .handle_play_packet(PlayClientbound::DisguisedChat(disguised_chat.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("disguised chat event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::DisguisedChat(update) if update == disguised_chat
        ));

        let system_chat = packets::SystemChat {
            content: "Welcome".to_string(),
            overlay: false,
        };
        stream
            .handle_play_packet(PlayClientbound::SystemChat(system_chat.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("system chat event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::SystemChat(update) if update == system_chat));

        let update_tags = packets::UpdateTags {
            registries: vec![packets::RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![packets::TagNetworkPayload {
                    tag: "minecraft:logs".to_string(),
                    entries: vec![5, 6, 7],
                }],
            }],
        };
        stream
            .handle_play_packet(PlayClientbound::UpdateTags(update_tags.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("update tags event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::UpdateTags(update) if update == update_tags));

        assert!(
            timeout(Duration::from_millis(50), server.read_packet())
                .await
                .is_err(),
            "command and chat client state packets must not send serverbound responses"
        );
    }

    #[tokio::test]
    async fn play_local_player_state_packets_emit_matching_events_without_responses() {
        let (client, mut server) = raw_connection_pair().await;
        let (events_tx, mut events_rx) = mpsc::channel(7);
        let (_commands_tx, commands_rx) = mpsc::channel(1);
        let mut stream = EventStreamContext {
            conn: client,
            events: events_tx,
            commands: commands_rx,
            state: ConnectionState::Play,
            player_loaded_sent: false,
            player_position_state: PlayerPositionState::default(),
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
        };

        let abilities = packets::PlayerAbilities {
            invulnerable: true,
            flying: false,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        };
        stream
            .handle_play_packet(PlayClientbound::PlayerAbilities(abilities))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("player abilities event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::PlayerAbilities(update) if update == abilities));

        let health = packets::PlayerHealth {
            health: 7.5,
            food: 16,
            saturation: 2.0,
        };
        stream
            .handle_play_packet(PlayClientbound::SetHealth(health))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("player health event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::PlayerHealth(update) if update == health));

        let experience = packets::PlayerExperience {
            progress: 0.75,
            level: 8,
            total: 123,
        };
        stream
            .handle_play_packet(PlayClientbound::SetExperience(experience))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("player experience event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::PlayerExperience(update) if update == experience
        ));

        let held_slot = packets::SetHeldSlot { slot: 5 };
        stream
            .handle_play_packet(PlayClientbound::SetHeldSlot(held_slot))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("held slot event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::HeldSlot(update) if update == held_slot));

        let spawn = packets::SetDefaultSpawnPosition {
            dimension: "minecraft:overworld".to_string(),
            pos: packets::BlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            yaw: 90.0,
            pitch: -10.0,
        };
        stream
            .handle_play_packet(PlayClientbound::SetDefaultSpawnPosition(spawn.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("default spawn event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::SetDefaultSpawnPosition(update) if update == spawn
        ));

        let simulation_distance = packets::SetSimulationDistance { distance: 12 };
        stream
            .handle_play_packet(PlayClientbound::SetSimulationDistance(simulation_distance))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("simulation distance event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::SetSimulationDistance(update) if update == simulation_distance
        ));

        let look_at = packets::PlayerLookAt {
            from_anchor: packets::EntityAnchor::Eyes,
            position: packets::Vec3d {
                x: 12.0,
                y: 65.0,
                z: -7.0,
            },
            target: Some(packets::PlayerLookAtTarget {
                entity_id: 99,
                to_anchor: packets::EntityAnchor::Feet,
            }),
        };
        stream
            .handle_play_packet(PlayClientbound::PlayerLookAt(look_at))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("player look-at event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::PlayerLookAt(update) if update == look_at));

        assert!(
            timeout(Duration::from_millis(50), server.read_packet())
                .await
                .is_err(),
            "local-player state packets must not send serverbound responses"
        );
    }

    #[tokio::test]
    async fn play_level_state_packets_emit_matching_events_without_responses() {
        let (client, mut server) = raw_connection_pair().await;
        let (events_tx, mut events_rx) = mpsc::channel(8);
        let (_commands_tx, commands_rx) = mpsc::channel(1);
        let mut stream = EventStreamContext {
            conn: client,
            events: events_tx,
            commands: commands_rx,
            state: ConnectionState::Play,
            player_loaded_sent: false,
            player_position_state: PlayerPositionState::default(),
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
        };

        let time = packets::PlayTime {
            game_time: 123,
            clock_updates: vec![packets::ClockUpdate {
                clock_id: 0,
                total_ticks: 6000,
                partial_tick: 0.25,
                rate: 1.0,
            }],
        };
        stream
            .handle_play_packet(PlayClientbound::SetTime(time.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("world time event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::SetTime(update) if update == time));

        let game_event = packets::GameEvent {
            event_id: 7,
            param: 0.5,
        };
        stream
            .handle_play_packet(PlayClientbound::GameEvent(game_event))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("game event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::GameEvent(update) if update == game_event));

        let ticking_state = packets::TickingState {
            tick_rate: 0.25,
            frozen: true,
        };
        stream
            .handle_play_packet(PlayClientbound::TickingState(ticking_state))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("ticking state event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::TickingState(update) if update == ticking_state
        ));

        let ticking_step = packets::TickingStep { tick_steps: 7 };
        stream
            .handle_play_packet(PlayClientbound::TickingStep(ticking_step))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("ticking step event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::TickingStep(update) if update == ticking_step));

        let ack = packets::BlockChangedAck { sequence: 17 };
        stream
            .handle_play_packet(PlayClientbound::BlockChangedAck(ack))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("block changed ack event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::BlockChangedAck(update) if update == ack));

        let block_entity = packets::BlockEntityData {
            pos: packets::BlockPos {
                x: 12,
                y: 65,
                z: -5,
            },
            block_entity_type_id: 4,
            raw_nbt: vec![10, 0],
        };
        stream
            .handle_play_packet(PlayClientbound::BlockEntityData(block_entity.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("block entity data event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::BlockEntityData(update) if update == block_entity
        ));

        let block_event = packets::BlockEvent {
            pos: packets::BlockPos {
                x: 12,
                y: 65,
                z: -5,
            },
            b0: 2,
            b1: 9,
            block_id: 54,
        };
        stream
            .handle_play_packet(PlayClientbound::BlockEvent(block_event))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("block event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::BlockEvent(update) if update == block_event));

        let level_event = packets::LevelEvent {
            event_type: 1001,
            pos: packets::BlockPos { x: 3, y: 4, z: 5 },
            data: 42,
            global: true,
        };
        stream
            .handle_play_packet(PlayClientbound::LevelEvent(level_event))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("level event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::LevelEvent(update) if update == level_event));

        assert!(
            timeout(Duration::from_millis(50), server.read_packet())
                .await
                .is_err(),
            "level state packets must not send serverbound responses"
        );
    }

    #[tokio::test]
    async fn play_chunk_terrain_packets_emit_matching_events_without_responses() {
        let (client, mut server) = raw_connection_pair().await;
        let (events_tx, mut events_rx) = mpsc::channel(8);
        let (_commands_tx, commands_rx) = mpsc::channel(1);
        let mut stream = EventStreamContext {
            conn: client,
            events: events_tx,
            commands: commands_rx,
            state: ConnectionState::Play,
            player_loaded_sent: false,
            player_position_state: PlayerPositionState::default(),
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
        };

        let chunk = packets::LevelChunkWithLight {
            x: 1,
            z: -2,
            chunk_data: packets::LevelChunkData {
                heightmaps: vec![packets::ChunkHeightmapData {
                    kind_id: 1,
                    data: vec![42],
                }],
                section_data: vec![0, 1, 2],
                block_entities: vec![packets::LevelChunkBlockEntity {
                    packed_xz: 0,
                    y: -64,
                    block_entity_type_id: 7,
                    raw_nbt: vec![0],
                }],
            },
            light_data: packets::LightUpdateData {
                sky_y_mask: Vec::new(),
                block_y_mask: Vec::new(),
                empty_sky_y_mask: Vec::new(),
                empty_block_y_mask: Vec::new(),
                sky_updates: Vec::new(),
                block_updates: Vec::new(),
            },
        };
        stream
            .handle_play_packet(PlayClientbound::LevelChunkWithLight(chunk.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("level chunk event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::LevelChunkWithLight(update) if update == chunk
        ));

        let light = packets::LightUpdate {
            chunk_x: 1,
            chunk_z: -2,
            light_data: packets::LightUpdateData {
                sky_y_mask: vec![0b10],
                block_y_mask: vec![0b10],
                empty_sky_y_mask: Vec::new(),
                empty_block_y_mask: Vec::new(),
                sky_updates: vec![vec![4; 2048]],
                block_updates: vec![vec![13; 2048]],
            },
        };
        stream
            .handle_play_packet(PlayClientbound::LightUpdate(light.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("light update event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::LightUpdate(update) if update == light));

        let biomes = packets::ChunksBiomes {
            chunks: vec![packets::ChunkBiomeData {
                pos: packets::ChunkPos { x: 1, z: -2 },
                raw_biomes: vec![0, 7],
            }],
        };
        stream
            .handle_play_packet(PlayClientbound::ChunksBiomes(biomes.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("chunk biome event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::ChunksBiomes(update) if update == biomes));

        let forget = packets::ForgetLevelChunk {
            pos: packets::ChunkPos { x: 1, z: -2 },
        };
        stream
            .handle_play_packet(PlayClientbound::ForgetLevelChunk(forget))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("chunk forget event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::ForgetLevelChunk(update) if update == forget
        ));

        let block = packets::BlockUpdate {
            pos: packets::BlockPos {
                x: 16,
                y: -64,
                z: -32,
            },
            block_state_id: 9,
        };
        stream
            .handle_play_packet(PlayClientbound::BlockUpdate(block))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("block update event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::BlockUpdate(update) if update == block));

        let section = packets::SectionBlocksUpdate {
            section_x: 1,
            section_y: -4,
            section_z: -2,
            updates: vec![
                packets::BlockUpdate {
                    pos: packets::BlockPos {
                        x: 17,
                        y: -64,
                        z: -32,
                    },
                    block_state_id: 9,
                },
                packets::BlockUpdate {
                    pos: packets::BlockPos {
                        x: 18,
                        y: -64,
                        z: -32,
                    },
                    block_state_id: 9,
                },
            ],
        };
        stream
            .handle_play_packet(PlayClientbound::SectionBlocksUpdate(section.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("section blocks update event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::SectionBlocksUpdate(update) if update == section
        ));

        let center = packets::SetChunkCacheCenter {
            chunk_x: 1,
            chunk_z: -2,
        };
        stream
            .handle_play_packet(PlayClientbound::SetChunkCacheCenter(center))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("chunk cache center event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::SetChunkCacheCenter(update) if update == center
        ));

        let radius = packets::SetChunkCacheRadius { radius: 7 };
        stream
            .handle_play_packet(PlayClientbound::SetChunkCacheRadius(radius))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("chunk cache radius event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::SetChunkCacheRadius(update) if update == radius
        ));

        assert!(
            timeout(Duration::from_millis(50), server.read_packet())
                .await
                .is_err(),
            "chunk terrain packets must not send serverbound responses"
        );
    }

    #[tokio::test]
    async fn play_debug_game_packets_emit_matching_events_without_responses() {
        let (client, mut server) = raw_connection_pair().await;
        let (events_tx, mut events_rx) = mpsc::channel(8);
        let (_commands_tx, commands_rx) = mpsc::channel(1);
        let mut stream = EventStreamContext {
            conn: client,
            events: events_tx,
            commands: commands_rx,
            state: ConnectionState::Play,
            player_loaded_sent: false,
            player_position_state: PlayerPositionState::default(),
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
        };

        let debug_block = packets::DebugBlockValue {
            pos: packets::BlockPos { x: 1, y: 64, z: -2 },
            raw_update_payload: vec![5, 1, 0xaa],
        };
        stream
            .handle_play_packet(PlayClientbound::DebugBlockValue(debug_block.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("debug block value event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::DebugBlockValue(update) if update == debug_block
        ));

        let debug_chunk = packets::DebugChunkValue {
            pos: packets::ChunkPos { x: 3, z: -4 },
            raw_update_payload: vec![7, 0],
        };
        stream
            .handle_play_packet(PlayClientbound::DebugChunkValue(debug_chunk.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("debug chunk value event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::DebugChunkValue(update) if update == debug_chunk
        ));

        let debug_entity = packets::DebugEntityValue {
            entity_id: 123,
            raw_update_payload: vec![9, 1, 0xbb],
        };
        stream
            .handle_play_packet(PlayClientbound::DebugEntityValue(debug_entity.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("debug entity value event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::DebugEntityValue(update) if update == debug_entity
        ));

        let debug_event = packets::DebugEvent {
            raw_event_payload: vec![4, 0xcc],
        };
        stream
            .handle_play_packet(PlayClientbound::DebugEvent(debug_event.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("debug event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::DebugEvent(update) if update == debug_event));

        let debug_sample = packets::DebugSample {
            sample: vec![100, -50],
            sample_type: packets::RemoteDebugSampleType::TickTime,
        };
        stream
            .handle_play_packet(PlayClientbound::DebugSample(debug_sample.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("debug sample event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::DebugSample(update) if update == debug_sample
        ));

        let game_rules = packets::GameRuleValues {
            values: vec![
                packets::GameRuleValue {
                    rule: "minecraft:do_daylight_cycle".to_string(),
                    value: "false".to_string(),
                },
                packets::GameRuleValue {
                    rule: "minecraft:random_tick_speed".to_string(),
                    value: "3".to_string(),
                },
            ],
        };
        stream
            .handle_play_packet(PlayClientbound::GameRuleValues(game_rules.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("game rule values event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::GameRuleValues(update) if update == game_rules
        ));

        let game_test_highlight = packets::GameTestHighlightPos {
            absolute_pos: packets::BlockPos {
                x: -10,
                y: 70,
                z: 22,
            },
            relative_pos: packets::BlockPos { x: 1, y: 2, z: 3 },
        };
        stream
            .handle_play_packet(PlayClientbound::GameTestHighlightPos(
                game_test_highlight.clone(),
            ))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("game test highlight event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::GameTestHighlightPos(update) if update == game_test_highlight
        ));

        let test_instance_status = packets::TestInstanceBlockStatus {
            status: "Ready".to_string(),
            size: Some(packets::Vec3i { x: 3, y: 4, z: 5 }),
        };
        stream
            .handle_play_packet(PlayClientbound::TestInstanceBlockStatus(
                test_instance_status.clone(),
            ))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("test instance block status event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::TestInstanceBlockStatus(update) if update == test_instance_status
        ));

        assert!(
            timeout(Duration::from_millis(50), server.read_packet())
                .await
                .is_err(),
            "debug/game-state packets must not send serverbound responses"
        );
    }

    #[tokio::test]
    async fn play_passive_world_apply_packets_emit_matching_events() {
        let (client, mut server) = raw_connection_pair().await;
        let (events_tx, mut events_rx) = mpsc::channel(8);
        let (_commands_tx, commands_rx) = mpsc::channel(1);
        let mut stream = EventStreamContext {
            conn: client,
            events: events_tx,
            commands: commands_rx,
            state: ConnectionState::Play,
            player_loaded_sent: false,
            player_position_state: PlayerPositionState::default(),
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
        };

        let projectile_power = packets::ProjectilePower {
            entity_id: 42,
            acceleration_power: 1.5,
        };
        stream
            .handle_play_packet(PlayClientbound::ProjectilePower(projectile_power))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("projectile power event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::ProjectilePower(update) if update == projectile_power
        ));

        let waypoint = packets::TrackedWaypointPacket {
            operation: packets::WaypointOperation::Track,
            waypoint: packets::TrackedWaypoint {
                identifier: packets::WaypointIdentifier::Name("base".to_string()),
                icon: packets::WaypointIcon {
                    style: "minecraft:default".to_string(),
                    color_rgb: Some(0x33_66_99),
                },
                data: packets::WaypointData::Position(packets::WaypointVec3i {
                    x: 12,
                    y: 64,
                    z: -8,
                }),
            },
        };
        stream
            .handle_play_packet(PlayClientbound::Waypoint(waypoint.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("waypoint event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::Waypoint(update) if update == waypoint));

        let recipe_id = packets::RecipeDisplayId { index: 7 };
        let recipe_book_add = packets::RecipeBookAdd {
            entries: vec![packets::RecipeBookAddEntry {
                contents: packets::RecipeDisplayEntry {
                    id: recipe_id,
                    display: packets::RecipeDisplaySummary {
                        display_type: packets::RecipeDisplayType::Stonecutter,
                        raw_body: vec![1, 2, 3],
                    },
                    group: Some(4),
                    category_id: 2,
                    crafting_requirements: Some(vec![packets::IngredientSummary {
                        tag: None,
                        item_ids: vec![3, 5],
                    }]),
                },
                flags: 0b11,
                notification: true,
                highlight: true,
            }],
            replace: true,
        };
        stream
            .handle_play_packet(PlayClientbound::RecipeBookAdd(recipe_book_add.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("recipe book add event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::RecipeBookAdd(update) if update == recipe_book_add
        ));

        let recipe_book_remove = packets::RecipeBookRemove {
            recipe_ids: vec![recipe_id],
        };
        stream
            .handle_play_packet(PlayClientbound::RecipeBookRemove(
                recipe_book_remove.clone(),
            ))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("recipe book remove event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::RecipeBookRemove(update) if update == recipe_book_remove
        ));

        let recipe_book_settings = packets::RecipeBookSettings {
            crafting: packets::RecipeBookTypeSettings {
                open: true,
                filtering: false,
            },
            furnace: packets::RecipeBookTypeSettings {
                open: false,
                filtering: true,
            },
            blast_furnace: packets::RecipeBookTypeSettings {
                open: true,
                filtering: true,
            },
            smoker: packets::RecipeBookTypeSettings {
                open: false,
                filtering: false,
            },
        };
        stream
            .handle_play_packet(PlayClientbound::RecipeBookSettings(recipe_book_settings))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("recipe book settings event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::RecipeBookSettings(update) if update == recipe_book_settings
        ));

        let update_advancements = packets::UpdateAdvancements {
            reset: true,
            added: Vec::new(),
            removed: vec!["minecraft:story/root".to_string()],
            progress: Vec::new(),
            show_advancements: true,
        };
        stream
            .handle_play_packet(PlayClientbound::UpdateAdvancements(
                update_advancements.clone(),
            ))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("advancements update event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::UpdateAdvancements(update) if update == update_advancements
        ));

        let select_advancements_tab = packets::SelectAdvancementsTab {
            tab: Some("minecraft:story/root".to_string()),
        };
        stream
            .handle_play_packet(PlayClientbound::SelectAdvancementsTab(
                select_advancements_tab.clone(),
            ))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("select advancements tab event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::SelectAdvancementsTab(update) if update == select_advancements_tab
        ));

        let update_recipes = packets::UpdateRecipes {
            property_sets: vec![packets::RecipePropertySetSummary {
                key: "minecraft:planks".to_string(),
                item_ids: vec![5, 6],
            }],
            stonecutter_recipes: vec![packets::StonecutterSelectableRecipeSummary {
                input: packets::IngredientSummary {
                    tag: Some("minecraft:stone_tool_materials".to_string()),
                    item_ids: vec![1],
                },
                option_display: packets::SlotDisplaySummary {
                    display_type_id: 2,
                    raw_payload: vec![9, 8, 7],
                },
            }],
        };
        stream
            .handle_play_packet(PlayClientbound::UpdateRecipes(update_recipes.clone()))
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("recipes update event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::UpdateRecipes(update) if update == update_recipes
        ));

        assert!(
            timeout(Duration::from_millis(50), server.read_packet())
                .await
                .is_err(),
            "passive world-apply packets must not send serverbound responses"
        );
    }

    #[tokio::test]
    async fn play_unknown_packets_emit_unsupported_packet_events() {
        let (client, mut server) = raw_connection_pair().await;
        let (events_tx, mut events_rx) = mpsc::channel(1);
        let (_commands_tx, commands_rx) = mpsc::channel(1);
        let mut stream = EventStreamContext {
            conn: client,
            events: events_tx,
            commands: commands_rx,
            state: ConnectionState::Play,
            player_loaded_sent: false,
            player_position_state: PlayerPositionState::default(),
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
        };

        stream
            .handle_play_packet(PlayClientbound::Unknown {
                packet_id: 0x7e,
                len: 9,
            })
            .await
            .unwrap();

        assert!(timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err());
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("unsupported packet event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::UnsupportedPacket {
                state: ConnectionState::Play,
                packet_id: 0x7e,
                len: 9,
            }
        ));
    }

    #[tokio::test]
    async fn play_bundle_delimiter_is_transport_noop() {
        let (client, mut server) = raw_connection_pair().await;
        let (events_tx, mut events_rx) = mpsc::channel(1);
        let (_commands_tx, commands_rx) = mpsc::channel(1);
        let mut stream = EventStreamContext {
            conn: client,
            events: events_tx,
            commands: commands_rx,
            state: ConnectionState::Play,
            player_loaded_sent: false,
            player_position_state: PlayerPositionState::default(),
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
        };

        stream
            .handle_play_packet(PlayClientbound::BundleDelimiter)
            .await
            .unwrap();

        assert!(timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err());
        assert!(timeout(Duration::from_millis(50), events_rx.recv())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn play_disconnect_packet_returns_disconnect_error() {
        let (client, mut server) = raw_connection_pair().await;
        let (events_tx, mut events_rx) = mpsc::channel(1);
        let (_commands_tx, commands_rx) = mpsc::channel(1);
        let mut stream = EventStreamContext {
            conn: client,
            events: events_tx,
            commands: commands_rx,
            state: ConnectionState::Play,
            player_loaded_sent: false,
            player_position_state: PlayerPositionState::default(),
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
        };

        let err = stream
            .handle_play_packet(PlayClientbound::Disconnect(packets::Disconnect {
                reason: "Kicked".to_string(),
                raw_reason: Vec::new(),
            }))
            .await
            .unwrap_err();

        assert_eq!(err.to_string(), "play disconnected: Kicked");
        assert!(timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err());
        assert!(timeout(Duration::from_millis(50), events_rx.recv())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn play_player_position_sends_loaded_after_first_position_sync_only() {
        let (client, mut server) = raw_connection_pair().await;
        let (events_tx, mut events_rx) = mpsc::channel(4);
        let (_commands_tx, commands_rx) = mpsc::channel(1);
        let mut stream = EventStreamContext {
            conn: client,
            events: events_tx,
            commands: commands_rx,
            state: ConnectionState::Play,
            player_loaded_sent: false,
            player_position_state: PlayerPositionState::default(),
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
        };
        let first_update = packets::PlayerPositionUpdate {
            id: 17,
            position: packets::Vec3d {
                x: 1.25,
                y: 64.5,
                z: -8.75,
            },
            delta_movement: packets::Vec3d::default(),
            y_rot: 90.0,
            x_rot: -15.0,
            relatives_mask: 0,
        };

        stream
            .handle_play_packet(PlayClientbound::PlayerPosition(first_update))
            .await
            .unwrap();

        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("player position event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::PlayerPosition(update) if update == first_update));

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("teleport ack should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_ACCEPT_TELEPORTATION);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 17);
        assert!(decoder.is_empty());

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("move player pos/rot ack should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_MOVE_PLAYER_POS_ROT);
        assert_move_player_pos_rot_payload(&payload, 1.25, 64.5, -8.75, 90.0, -15.0, 0);

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("player loaded should be sent after first position sync")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_PLAYER_LOADED);
        assert!(payload.is_empty());
        assert!(stream.player_loaded_sent);

        let second_update = packets::PlayerPositionUpdate {
            id: 18,
            position: packets::Vec3d {
                x: 2.0,
                y: 70.0,
                z: -9.0,
            },
            delta_movement: packets::Vec3d::default(),
            y_rot: 100.0,
            x_rot: 5.0,
            relatives_mask: 0,
        };

        stream
            .handle_play_packet(PlayClientbound::PlayerPosition(second_update))
            .await
            .unwrap();

        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("second player position event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::PlayerPosition(update) if update == second_update));

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("second teleport ack should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_ACCEPT_TELEPORTATION);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 18);
        assert!(decoder.is_empty());

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("second move player pos/rot ack should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_MOVE_PLAYER_POS_ROT);
        assert_move_player_pos_rot_payload(&payload, 2.0, 70.0, -9.0, 100.0, 5.0, 0);
        assert!(
            timeout(Duration::from_millis(50), server.read_packet())
                .await
                .is_err(),
            "player loaded must only be sent for the first position sync"
        );
    }

    #[tokio::test]
    async fn play_player_rotation_sends_vanilla_rot_ack() {
        let (client, mut server) = raw_connection_pair().await;
        let (events_tx, mut events_rx) = mpsc::channel(4);
        let (_commands_tx, commands_rx) = mpsc::channel(1);
        let mut stream = EventStreamContext {
            conn: client,
            events: events_tx,
            commands: commands_rx,
            state: ConnectionState::Play,
            player_loaded_sent: true,
            player_position_state: PlayerPositionState {
                y_rot: 30.0,
                x_rot: 5.0,
                ..PlayerPositionState::default()
            },
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
        };
        let update = packets::PlayerRotationUpdate {
            y_rot: 15.0,
            relative_y: true,
            x_rot: -10.0,
            relative_x: false,
        };

        stream
            .handle_play_packet(PlayClientbound::PlayerRotation(update))
            .await
            .unwrap();

        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("player rotation event should be emitted")
            .unwrap();
        assert!(matches!(event, NetEvent::PlayerRotation(event_update) if event_update == update));

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("move player rot ack should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_MOVE_PLAYER_ROT);
        assert_move_player_rot_payload(&payload, 45.0, -10.0, 0);
        assert_eq!(stream.player_position_state.y_rot, 45.0);
        assert_eq!(stream.player_position_state.x_rot, -10.0);
    }

    #[tokio::test]
    async fn play_start_configuration_acknowledges_and_resets_configuration_dedup_state() {
        let (client, mut server) = raw_connection_pair().await;
        let (events_tx, mut events_rx) = mpsc::channel(4);
        let (_commands_tx, commands_rx) = mpsc::channel(1);
        let stream = EventStreamContext {
            conn: client,
            events: events_tx,
            commands: commands_rx,
            state: ConnectionState::Play,
            player_loaded_sent: false,
            player_position_state: PlayerPositionState::default(),
            play_tick: Some(crate::connection::play_tick_interval()),
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: true,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
        };

        let stream_task = tokio::spawn(async move {
            let mut stream = stream;
            stream
                .handle_play_packet(PlayClientbound::StartConfiguration)
                .await
                .unwrap();
            stream
        });

        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("start-configuration event should be emitted")
            .unwrap();
        match event {
            NetEvent::StartConfiguration {
                pending_chat_acknowledgement,
            } => {
                pending_chat_acknowledgement
                    .send(None)
                    .expect("start-configuration handler should wait for reply");
            }
            other => panic!("expected start-configuration event, got {other:?}"),
        }

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("configuration acknowledgement should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_CONFIGURATION_ACKNOWLEDGED);
        assert!(payload.is_empty());
        let mut stream = timeout(Duration::from_secs(1), stream_task)
            .await
            .expect("start-configuration task should finish")
            .unwrap();
        assert_eq!(stream.state, ConnectionState::Configuration);
        assert!(stream.play_tick.is_none());
        assert!(!stream.seen_code_of_conduct);

        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("state-changed event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::StateChanged {
                state: ConnectionState::Configuration
            }
        ));

        stream
            .handle_configuration_packet(packets::ConfigurationClientbound::CodeOfConduct {
                text: "Fresh configuration rules.".to_string(),
            })
            .await
            .unwrap();
        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("code-of-conduct event should be emitted after reconfiguration")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::CodeOfConduct { text } if text == "Fresh configuration rules."
        ));
    }

    #[tokio::test]
    async fn play_start_configuration_flushes_pending_chat_acknowledgement_first() {
        let (client, mut server) = raw_connection_pair().await;
        let (events_tx, mut events_rx) = mpsc::channel(4);
        let (_commands_tx, commands_rx) = mpsc::channel(1);
        let stream = EventStreamContext {
            conn: client,
            events: events_tx,
            commands: commands_rx,
            state: ConnectionState::Play,
            player_loaded_sent: false,
            player_position_state: PlayerPositionState::default(),
            play_tick: Some(crate::connection::play_tick_interval()),
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: true,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
        };

        let stream_task = tokio::spawn(async move {
            let mut stream = stream;
            stream
                .handle_play_packet(PlayClientbound::StartConfiguration)
                .await
                .unwrap();
            stream
        });

        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("start-configuration event should be emitted")
            .unwrap();
        match event {
            NetEvent::StartConfiguration {
                pending_chat_acknowledgement,
            } => {
                pending_chat_acknowledgement
                    .send(Some(ChatAcknowledgement { offset: 1 }))
                    .expect("start-configuration handler should wait for reply");
            }
            other => panic!("expected start-configuration event, got {other:?}"),
        }

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("chat acknowledgement should be sent first")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_CHAT_ACK);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 1);
        assert!(decoder.is_empty());

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("configuration acknowledgement should be sent second")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_CONFIGURATION_ACKNOWLEDGED);
        assert!(payload.is_empty());

        let stream = timeout(Duration::from_secs(1), stream_task)
            .await
            .expect("start-configuration task should finish")
            .unwrap();
        assert_eq!(stream.state, ConnectionState::Configuration);

        let event = timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .expect("state-changed event should be emitted")
            .unwrap();
        assert!(matches!(
            event,
            NetEvent::StateChanged {
                state: ConnectionState::Configuration
            }
        ));
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
        let server = RawConnection {
            stream: server_stream,
            read_buf: BytesMut::with_capacity(8192),
            compression_threshold: None,
        };
        (client, server)
    }

    fn test_item_stack(item_id: i32, count: i32) -> packets::ItemStackSummary {
        packets::ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: packets::DataComponentPatchSummary::default(),
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
                        type_id: 20,
                        name: "minecraft:message".to_string(),
                        properties: vec![2],
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
            target_name: None,
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
}
