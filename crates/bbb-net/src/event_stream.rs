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
    player_was_dead: bool,
    play_tick: Option<Interval>,
    chunk_batch_size: ChunkBatchSizeCalculator,
    server_cookies: BTreeMap<String, Vec<u8>>,
    seen_code_of_conduct: bool,
    accepted_code_of_conduct_hash: Option<i32>,
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
        player_was_dead: false,
        play_tick: None,
        chunk_batch_size: ChunkBatchSizeCalculator::new(),
        server_cookies: BTreeMap::new(),
        seen_code_of_conduct: false,
        accepted_code_of_conduct_hash: options.accepted_code_of_conduct_hash,
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
        packets::{LoginClientbound, PlayClientbound},
    };
    use bbb_world::code_of_conduct_text_hash;
    use bytes::BytesMut;
    use std::time::Duration;
    use tokio::{net::TcpListener, time::timeout};

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
            player_was_dead: false,
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
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
            player_was_dead: false,
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: Some(code_of_conduct_text_hash(text)),
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
            player_was_dead: false,
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
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
            player_was_dead: false,
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
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
            player_was_dead: false,
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
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
            player_was_dead: false,
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
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
            player_was_dead: false,
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
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
            player_was_dead: false,
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
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
            player_was_dead: false,
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
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
    async fn play_start_configuration_acknowledges_and_resets_configuration_dedup_state() {
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
            player_was_dead: false,
            play_tick: Some(crate::connection::play_tick_interval()),
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: true,
            accepted_code_of_conduct_hash: None,
        };

        stream
            .handle_play_packet(PlayClientbound::StartConfiguration)
            .await
            .unwrap();

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("configuration acknowledgement should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_CONFIGURATION_ACKNOWLEDGED);
        assert!(payload.is_empty());
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
}
