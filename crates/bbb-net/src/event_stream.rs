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
