use std::{mem::size_of, sync::Arc, time::Duration};

use bbb_protocol::packets::{ClientInformation, GameProfile};
use tokio::{net::TcpStream, sync::watch, time::Instant};

use crate::{
    mailbox::{
        bounded_mailbox, MailboxReceive, MailboxReceiver, MailboxSendError, MailboxSender,
        MailboxTrySendError, RetainedBytes,
    },
    session::{
        ConnectionCloseReason, ConnectionLimits, ConnectionPhase, ServerConnectionConfig,
        ServerSession, SessionStep,
    },
    transport::{RawTransport, COALESCE_WRITE_BYTES},
    Result, ServerNetError,
};

// `ServerConnectionListener.java:67` (`new ReadTimeoutHandler(30)`).
const READ_IDLE_TIMEOUT: Duration = Duration::from_secs(30);
// ServerLoginPacketListenerImpl.MAX_TICKS_BEFORE_LOGIN: 600 ticks at 20 TPS.
const LOGIN_TIMEOUT: Duration = Duration::from_secs(30);
// ServerCommonPacketListenerImpl.LATENCY_CHECK_INTERVAL (`ServerCommonPacketListenerImpl.java:35`).
const CONFIGURATION_KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionEvent {
    PhaseChanged {
        phase: ConnectionPhase,
    },
    CustomClickAction(bbb_protocol::packets::CustomClickAction),
    PlayReady {
        transferred: bool,
        profile: GameProfile,
        client_information: ClientInformation,
        client_brand: Option<String>,
    },
    PlayPacket {
        packet_id: i32,
        payload: Vec<u8>,
    },
}

impl RetainedBytes for ConnectionEvent {
    fn retained_bytes(&self) -> usize {
        match self {
            Self::PhaseChanged { .. } => size_of::<Self>(),
            Self::CustomClickAction(action) => size_of::<Self>() + action.id.capacity(),
            Self::PlayReady {
                profile,
                client_information,
                client_brand,
                ..
            } => {
                size_of::<Self>()
                    + profile.name.capacity()
                    + profile.properties.capacity()
                        * size_of::<bbb_protocol::packets::GameProfileProperty>()
                    + profile
                        .properties
                        .iter()
                        .map(|property| {
                            property.name.capacity()
                                + property.value.capacity()
                                + property.signature.as_ref().map_or(0, String::capacity)
                        })
                        .sum::<usize>()
                    + client_information.language.capacity()
                    + client_brand.as_ref().map_or(0, String::capacity)
            }
            Self::PlayPacket { payload, .. } => size_of::<Self>() + payload.capacity(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PreparedPlayPacket {
    packet_id: i32,
    compression_threshold: Option<i32>,
    framed: Arc<[u8]>,
}

impl PreparedPlayPacket {
    pub fn encode(
        packet_id: i32,
        payload: Arc<Vec<u8>>,
        compression_threshold: Option<i32>,
    ) -> std::result::Result<Self, bbb_protocol::codec::ProtocolError> {
        let framed = bbb_protocol::frame::with_thread_local_encoder(|encoder| {
            encoder
                .encode(packet_id, &payload, compression_threshold)
                .map(Arc::<[u8]>::from)
        })?;
        drop(payload);
        Ok(Self {
            packet_id,
            compression_threshold,
            framed,
        })
    }

    pub fn packet_id(&self) -> i32 {
        self.packet_id
    }

    pub fn compression_threshold(&self) -> Option<i32> {
        self.compression_threshold
    }

    pub fn framed(&self) -> &[u8] {
        &self.framed
    }

    pub fn framed_shared(&self) -> &Arc<[u8]> {
        &self.framed
    }
}

#[derive(Debug, Clone)]
pub enum ConnectionCommand {
    // Keep the encoder's allocation when sharing a broadcast across routes.
    SendPlayPacket {
        packet_id: i32,
        payload: Arc<Vec<u8>>,
    },
    SendPreparedPlayPacket(PreparedPlayPacket),
    StartConfiguration {
        config: Option<Arc<ServerConnectionConfig>>,
    },
    Close,
}

impl RetainedBytes for ConnectionCommand {
    fn retained_bytes(&self) -> usize {
        match self {
            Self::SendPlayPacket { payload, .. } => size_of::<Self>() + payload.len(),
            Self::SendPreparedPlayPacket(prepared) => size_of::<Self>() + prepared.framed().len(),
            Self::StartConfiguration { .. } | Self::Close => size_of::<Self>(),
        }
    }
}

pub struct ConnectionHandle {
    events: MailboxReceiver<ConnectionEvent>,
    commands: MailboxSender<ConnectionCommand>,
}

#[derive(Clone)]
pub struct ConnectionCommandSender {
    commands: MailboxSender<ConnectionCommand>,
}

impl ConnectionHandle {
    pub async fn next_event(&self) -> Option<ConnectionEvent> {
        self.events.recv().await
    }

    pub fn try_send(
        &self,
        command: ConnectionCommand,
    ) -> std::result::Result<(), MailboxTrySendError<ConnectionCommand>> {
        self.commands.try_send(command)
    }

    pub async fn send(
        &self,
        command: ConnectionCommand,
    ) -> std::result::Result<(), MailboxSendError<ConnectionCommand>> {
        self.commands.send(command).await
    }

    pub fn command_sender(&self) -> ConnectionCommandSender {
        ConnectionCommandSender {
            commands: self.commands.clone(),
        }
    }

    pub fn close(&self) {
        self.events.close();
        self.commands.close();
    }
}

impl ConnectionCommandSender {
    pub fn try_send(
        &self,
        command: ConnectionCommand,
    ) -> std::result::Result<(), MailboxTrySendError<ConnectionCommand>> {
        self.commands.try_send(command)
    }

    pub async fn send(
        &self,
        command: ConnectionCommand,
    ) -> std::result::Result<(), MailboxSendError<ConnectionCommand>> {
        self.commands.send(command).await
    }

    pub fn close(&self) {
        self.commands.close();
    }

    pub fn is_closed(&self) -> bool {
        self.commands.is_closed()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionOutcome {
    pub final_phase: ConnectionPhase,
    pub reason: ConnectionCloseReason,
}

pub struct ServerConnectionDriver {
    transport: RawTransport,
    session: ServerSession,
    events: MailboxSender<ConnectionEvent>,
    commands: MailboxReceiver<ConnectionCommand>,
    idle_timeout: Duration,
    keep_alive_interval: Duration,
    live_config: Option<watch::Receiver<Arc<ServerConnectionConfig>>>,
}

impl ServerConnectionDriver {
    pub(crate) fn new(
        stream: TcpStream,
        config: Arc<ServerConnectionConfig>,
        limits: ConnectionLimits,
    ) -> Result<(ConnectionHandle, Self)> {
        let limits = limits
            .validate()
            .map_err(ServerNetError::InvalidConfiguration)?;
        let session = ServerSession::new(config, stream.peer_addr().ok())?;
        let (events_tx, events_rx) = bounded_mailbox(limits.event_mailbox());
        let (commands_tx, commands_rx) = bounded_mailbox(limits.command_mailbox());
        Ok((
            ConnectionHandle {
                events: events_rx,
                commands: commands_tx,
            },
            Self {
                transport: RawTransport::new(stream),
                session,
                events: events_tx,
                commands: commands_rx,
                idle_timeout: READ_IDLE_TIMEOUT,
                keep_alive_interval: CONFIGURATION_KEEP_ALIVE_INTERVAL,
                live_config: None,
            },
        ))
    }

    pub fn with_live_config(
        mut self,
        live_config: watch::Receiver<Arc<ServerConnectionConfig>>,
    ) -> Self {
        self.live_config = Some(live_config);
        self
    }

    #[cfg(test)]
    pub(crate) fn with_idle_timeout(mut self, timeout: Duration) -> Self {
        self.idle_timeout = timeout;
        self
    }

    #[cfg(test)]
    pub(crate) fn with_keep_alive_interval(mut self, interval: Duration) -> Self {
        self.keep_alive_interval = interval;
        self
    }

    pub async fn run(mut self) -> Result<ConnectionOutcome> {
        let closed = self.commands.closed();
        tokio::select! {
            biased;
            () = closed => self.finish(ConnectionCloseReason::CommandProducerClosed).await,
            outcome = self.run_active() => outcome,
        }
    }

    async fn run_active(&mut self) -> Result<ConnectionOutcome> {
        let idle_timeout_duration = self.idle_timeout;
        let idle_timeout = tokio::time::sleep(idle_timeout_duration);
        tokio::pin!(idle_timeout);
        let login_timeout = tokio::time::sleep(LOGIN_TIMEOUT);
        tokio::pin!(login_timeout);
        let keep_alive_interval = self.keep_alive_interval;
        let configuration_keep_alive = tokio::time::sleep(keep_alive_interval);
        tokio::pin!(configuration_keep_alive);
        loop {
            let step = tokio::select! {
                biased;
                command = self.commands.recv() => {
                    if let Some(outcome) = self.apply_command(command).await? {
                        return Ok(outcome);
                    }
                    continue;
                }
                _ = &mut idle_timeout, if matches!(
                    self.session.phase(),
                    ConnectionPhase::Handshake | ConnectionPhase::Status
                ) => {
                    return self.finish(ConnectionCloseReason::IdleTimeout).await;
                }
                _ = &mut login_timeout, if self.session.phase() == ConnectionPhase::Login => {
                    ServerSession::login_timeout()
                }
                _ = &mut configuration_keep_alive, if self.session.phase() == ConnectionPhase::Configuration => {
                    configuration_keep_alive
                        .as_mut()
                        .reset(Instant::now() + keep_alive_interval);
                    self.session.configuration_keep_alive(epoch_millis())
                }
                packet = self.transport.read_packet() => {
                    match packet {
                        Ok(Some((packet_id, payload))) => {
                            let phase_before = self.session.phase();
                            if phase_before == ConnectionPhase::Handshake {
                                login_timeout.as_mut().reset(Instant::now() + LOGIN_TIMEOUT);
                            }
                            let step = self.session.handle_packet(packet_id, payload);
                            let phase_after = self.session.phase();
                            if matches!(
                                phase_after,
                                ConnectionPhase::Handshake | ConnectionPhase::Status
                            ) {
                                idle_timeout
                                    .as_mut()
                                    .reset(Instant::now() + idle_timeout_duration);
                            }
                            if phase_before != ConnectionPhase::Configuration
                                && phase_after == ConnectionPhase::Configuration
                            {
                                configuration_keep_alive
                                    .as_mut()
                                    .reset(Instant::now() + keep_alive_interval);
                            }
                            step
                        }
                        Ok(None) => return self.finish(ConnectionCloseReason::RemoteClosed).await,
                        Err(ServerNetError::Protocol(error)) => {
                            return self.finish(ConnectionCloseReason::ProtocolViolation(error.to_string())).await;
                        }
                        Err(error) => return Err(error),
                    }
                }
            };

            let step = match step {
                Ok(step) => step,
                Err(ServerNetError::Protocol(error)) => {
                    return self
                        .finish(ConnectionCloseReason::ProtocolViolation(error.to_string()))
                        .await;
                }
                Err(error) => return Err(error),
            };
            if let Some(outcome) = self.apply(step).await? {
                return Ok(outcome);
            }
        }
    }

    async fn apply(&mut self, step: SessionStep) -> Result<Option<ConnectionOutcome>> {
        for packet in step.outbound {
            self.transport.send(packet).await?;
        }
        let events = self.events.clone();
        for event in step.events {
            let send = events.send(event);
            tokio::pin!(send);
            loop {
                // Inbound backpressure must not stop outbound delivery: the
                // tick can be waiting for that delivery before taking input.
                tokio::select! {
                    biased;
                    command = self.commands.recv() => {
                        if let Some(outcome) = self.apply_command(command).await? {
                            return Ok(Some(outcome));
                        }
                    }
                    sent = &mut send => {
                        if sent.is_err() {
                            return self
                                .finish(ConnectionCloseReason::EventConsumerClosed)
                                .await
                                .map(Some);
                        }
                        break;
                    }
                }
            }
        }
        if let Some(reason) = step.close {
            return self.finish(reason).await.map(Some);
        }
        Ok(None)
    }

    async fn apply_command(
        &mut self,
        mut command: Option<ConnectionCommand>,
    ) -> Result<Option<ConnectionOutcome>> {
        let mut coalesced_bytes = 0usize;
        let mut close_reason = None;
        loop {
            match command {
                Some(ConnectionCommand::Close) => {
                    close_reason = Some(ConnectionCloseReason::ServerCommand);
                    break;
                }
                Some(ConnectionCommand::SendPlayPacket { packet_id, payload }) => {
                    if !self.session.is_reconfiguring() {
                        if self.session.phase() == ConnectionPhase::Play {
                            coalesced_bytes = coalesced_bytes.saturating_add(
                                self.transport
                                    .buffer_or_send_play_packet(packet_id, &payload)
                                    .await?,
                            );
                        } else {
                            close_reason = Some(ConnectionCloseReason::ProtocolViolation(
                                "server attempted to send a play packet before Play phase"
                                    .to_string(),
                            ));
                            break;
                        }
                    }
                }
                Some(ConnectionCommand::SendPreparedPlayPacket(prepared)) => {
                    if !self.session.is_reconfiguring() {
                        if self.session.phase() == ConnectionPhase::Play {
                            coalesced_bytes = coalesced_bytes.saturating_add(
                                self.transport
                                    .buffer_or_send_prepared_play_packet(&prepared)
                                    .await?,
                            );
                        } else {
                            close_reason = Some(ConnectionCloseReason::ProtocolViolation(
                                "server attempted to send a play packet before Play phase"
                                    .to_string(),
                            ));
                            break;
                        }
                    }
                }
                Some(ConnectionCommand::StartConfiguration { config }) => {
                    self.transport.flush_writes().await?;
                    let next_config =
                        config.or_else(|| self.live_config.as_ref().map(|rx| rx.borrow().clone()));
                    match self.session.begin_reconfiguration(next_config) {
                        Ok(step) => {
                            debug_assert!(step.events.is_empty() && step.close.is_none());
                            for packet in step.outbound {
                                self.transport.send(packet).await?;
                            }
                            coalesced_bytes = 0;
                        }
                        Err(ServerNetError::Protocol(error)) => {
                            close_reason =
                                Some(ConnectionCloseReason::ProtocolViolation(error.to_string()));
                            break;
                        }
                        Err(error) => return Err(error),
                    }
                }
                None => {
                    close_reason = Some(ConnectionCloseReason::CommandProducerClosed);
                    break;
                }
            }

            if coalesced_bytes >= COALESCE_WRITE_BYTES {
                break;
            }
            match self.commands.try_recv() {
                MailboxReceive::Item(next) => command = Some(next),
                MailboxReceive::Empty | MailboxReceive::Closed => break,
            }
        }

        self.transport.flush_writes().await?;
        if let Some(reason) = close_reason {
            return self.finish(reason).await.map(Some);
        }
        Ok(None)
    }

    async fn finish(&mut self, reason: ConnectionCloseReason) -> Result<ConnectionOutcome> {
        self.transport.shutdown().await;
        self.events.close();
        self.commands.close();
        Ok(ConnectionOutcome {
            final_phase: self.session.phase(),
            reason,
        })
    }
}

fn epoch_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::{
        frame::encode_packet,
        packets::{
            encode_configuration_finish, encode_handshake, encode_login_acknowledged,
            encode_login_hello, encode_select_known_packs, ClientIntent,
        },
    };
    use tokio::{io::AsyncReadExt, net::TcpListener, time::timeout};

    fn enter_play(driver: &mut ServerConnectionDriver) {
        for packet in [
            encode_handshake("localhost", 25565, ClientIntent::Login).unwrap(),
            encode_login_hello("slow-reader", uuid::Uuid::nil()).unwrap(),
        ] {
            driver.session.handle_packet(packet.0, &packet.1).unwrap();
        }
        for packet in [
            encode_login_acknowledged(),
            encode_select_known_packs(&[]).unwrap(),
            encode_configuration_finish(),
        ] {
            driver.session.handle_packet(packet.0, &packet.1).unwrap();
        }
        assert_eq!(driver.session.phase(), ConnectionPhase::Play);
    }

    #[tokio::test]
    async fn closing_the_route_interrupts_a_blocked_tcp_write() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let _client = TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        let (stream, _) = listener.accept().await.unwrap();
        let (handle, mut driver) = ServerConnectionDriver::new(
            stream,
            Arc::new(ServerConnectionConfig::new(serde_json::json!({}))),
            ConnectionLimits {
                command_entries: 1,
                ..ConnectionLimits::default()
            },
        )
        .unwrap();
        enter_play(&mut driver);
        let task = tokio::spawn(driver.run());
        let route = handle.command_sender();
        let payload = Arc::new(vec![0; 512 * 1024]);
        let fill = async {
            for _ in 0..128 {
                route
                    .send(ConnectionCommand::SendPlayPacket {
                        packet_id: 42,
                        payload: Arc::clone(&payload),
                    })
                    .await
                    .unwrap();
            }
        };
        assert!(
            timeout(Duration::from_millis(100), fill).await.is_err(),
            "the unread TCP stream backpressures the route"
        );
        route.close();
        let outcome = timeout(Duration::from_secs(1), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(outcome.reason, ConnectionCloseReason::CommandProducerClosed);
        assert_eq!(outcome.final_phase, ConnectionPhase::Play);
    }

    #[tokio::test]
    async fn a_queued_normal_close_cannot_keep_a_stalled_writer_alive() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let _client = TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        let (stream, _) = listener.accept().await.unwrap();
        let (handle, mut driver) = ServerConnectionDriver::new(
            stream,
            Arc::new(ServerConnectionConfig::new(serde_json::json!({}))),
            ConnectionLimits::default(),
        )
        .unwrap();
        enter_play(&mut driver);
        let task = tokio::spawn(driver.run());
        let route = handle.command_sender();
        let payload = Arc::new(vec![0; 512 * 1024]);
        let fill = async {
            for _ in 0..128 {
                route
                    .send(ConnectionCommand::SendPlayPacket {
                        packet_id: 42,
                        payload: Arc::clone(&payload),
                    })
                    .await
                    .unwrap();
            }
        };
        assert!(timeout(Duration::from_millis(100), fill).await.is_err());
        route.try_send(ConnectionCommand::Close).unwrap();
        let result = timeout(Duration::from_secs(15), task)
            .await
            .expect("the queued close cannot leave a stalled connection running")
            .unwrap();
        assert!(
            matches!(result, Err(ServerNetError::Io(error)) if error.kind() == std::io::ErrorKind::TimedOut)
        );
        assert!(handle.next_event().await.is_none());
    }

    #[tokio::test]
    async fn a_full_inbound_queue_still_accepts_an_outbound_close() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut client = TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        let (stream, _) = listener.accept().await.unwrap();
        let (handle, mut driver) = ServerConnectionDriver::new(
            stream,
            Arc::new(ServerConnectionConfig::new(serde_json::json!({}))),
            ConnectionLimits {
                event_entries: 1,
                ..ConnectionLimits::default()
            },
        )
        .unwrap();
        enter_play(&mut driver);
        driver
            .events
            .send(ConnectionEvent::PhaseChanged {
                phase: ConnectionPhase::Login,
            })
            .await
            .unwrap();
        let step = SessionStep {
            events: vec![ConnectionEvent::PhaseChanged {
                phase: ConnectionPhase::Configuration,
            }],
            ..SessionStep::default()
        };
        let pending = driver.apply(step);
        tokio::pin!(pending);
        tokio::select! {
            outcome = &mut pending => panic!("event queue should remain full: {outcome:?}"),
            _ = tokio::task::yield_now() => {}
        }
        handle
            .send(ConnectionCommand::SendPlayPacket {
                packet_id: 42,
                payload: Arc::new(b"still delivering".to_vec()),
            })
            .await
            .unwrap();
        let expected = encode_packet(42, b"still delivering");
        let mut packet = vec![0; expected.len()];
        timeout(Duration::from_secs(1), async {
            tokio::select! {
                outcome = &mut pending => panic!("the inbound event is still backpressured: {outcome:?}"),
                read = client.read_exact(&mut packet) => { read.unwrap(); }
            }
        }).await.unwrap();
        assert_eq!(packet, expected);
        handle.send(ConnectionCommand::Close).await.unwrap();
        let outcome = timeout(Duration::from_secs(1), pending)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(outcome.reason, ConnectionCloseReason::ServerCommand);
        assert!(matches!(
            handle.next_event().await,
            Some(ConnectionEvent::PhaseChanged {
                phase: ConnectionPhase::Login,
            })
        ));
        assert!(handle.next_event().await.is_none());
        assert_eq!(client.read(&mut [0]).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn prepared_play_packet_drops_uncompressed_buffer_and_charges_wire_frame_quota() {
        let payload = Arc::new(vec![0x3a; 64 * 1024]);
        let weak = Arc::downgrade(&payload);
        let prepared = PreparedPlayPacket::encode(42, payload, Some(256)).unwrap();
        assert!(
            weak.upgrade().is_none(),
            "PreparedPlayPacket must not retain the uncompressed payload buffer"
        );
        assert!(prepared.framed().len() < 1024);

        let cmd = ConnectionCommand::SendPreparedPlayPacket(prepared.clone());
        let expected_bytes = size_of::<ConnectionCommand>() + prepared.framed().len();
        assert_eq!(cmd.retained_bytes(), expected_bytes);

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut client = TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        let (stream, _) = listener.accept().await.unwrap();
        let mut cfg = ServerConnectionConfig::new(serde_json::json!({}));
        cfg.compression_threshold = Some(256);
        let (handle, mut driver) = ServerConnectionDriver::new(
            stream,
            Arc::new(cfg),
            ConnectionLimits {
                command_entries: 4,
                command_bytes: expected_bytes,
                ..ConnectionLimits::default()
            },
        )
        .unwrap();
        enter_play(&mut driver);
        driver
            .transport
            .set_compression_threshold_for_test(Some(256));
        let task = tokio::spawn(driver.run());

        // Fits the 1-frame wire quota even though the uncompressed payload was 64 KiB.
        handle.try_send(cmd.clone()).unwrap();
        // A second prepared packet exceeds the exact 1-frame wire quota.
        assert!(matches!(
            handle.try_send(cmd),
            Err(MailboxTrySendError::Full(_))
        ));

        let mut received = vec![0u8; prepared.framed().len()];
        timeout(Duration::from_secs(1), client.read_exact(&mut received))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(received, prepared.framed());

        handle.send(ConnectionCommand::Close).await.unwrap();
        let outcome = timeout(Duration::from_secs(1), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(outcome.reason, ConnectionCloseReason::ServerCommand);
    }
}
