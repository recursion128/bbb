use std::{collections::BTreeSet, time::Duration};

use anyhow::{anyhow, Result};
use bbb_protocol::frame::{PacketDecoder, PacketEncoder};
use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::transport::{connect_transport, BoxedClientStream, ClientStream};

/// Vanilla Netty `ReadTimeoutHandler(30)` post-connect idle read ceiling
/// (`Connection.java`).
pub const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(30);

#[cfg(not(target_arch = "wasm32"))]
pub fn play_tick_interval() -> tokio::time::Interval {
    let mut tick = tokio::time::interval(Duration::from_millis(50));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    tick
}

const READ_BUF_RESERVE_BYTES: usize = 65_536;

#[derive(Clone, Copy)]
struct WallMark {
    instant: bbb_client_clock::Instant,
}

impl WallMark {
    fn now() -> Self {
        Self {
            instant: bbb_client_clock::Instant::now(),
        }
    }

    fn elapsed(self) -> Duration {
        self.instant.elapsed()
    }
}

/// Plaintext-only transport: online mode is a declared non-goal, so the vanilla
/// AES/CFB8 stream ciphers are intentionally absent and a login encryption
/// request fails the connection before any cipher could be negotiated.
pub struct RawConnection {
    pub(crate) stream: BoxedClientStream,
    pub(crate) srv_redirect: Option<(String, u16)>,
    pub(crate) read_buf: BytesMut,
    decoder: PacketDecoder,
    encoder: PacketEncoder,
    pub(crate) compression_threshold: Option<i32>,
    read_timeout: Option<Duration>,
    last_inbound_at: WallMark,
    login_hello_sent: bool,
    serverbound_carried_hotbar_slot: Option<u8>,
    in_play_phase: bool,
    serverbound_play_ids: BTreeSet<i32>,
    pending_flush: bool,
}

impl RawConnection {
    pub async fn connect(address: &str, compression_threshold: Option<i32>) -> Result<Self> {
        Self::connect_with_gateway(
            address,
            None,
            Duration::from_secs(20),
            compression_threshold,
        )
        .await
    }

    pub async fn connect_with_gateway(
        address: &str,
        websocket_gateway: Option<&str>,
        timeout_duration: Duration,
        compression_threshold: Option<i32>,
    ) -> Result<Self> {
        let (stream, redirect) =
            connect_transport(address, websocket_gateway, timeout_duration).await?;
        Ok(Self::from_boxed_stream(
            stream,
            redirect,
            compression_threshold,
        ))
    }

    /// Creates the transport owner around an already-connected stream in loopback tests.
    pub fn from_stream(
        stream: impl ClientStream + 'static,
        compression_threshold: Option<i32>,
    ) -> Self {
        Self::from_boxed_stream(Box::new(stream), None, compression_threshold)
    }

    pub(crate) fn from_boxed_stream(
        stream: BoxedClientStream,
        srv_redirect: Option<(String, u16)>,
        compression_threshold: Option<i32>,
    ) -> Self {
        Self {
            stream,
            srv_redirect,
            read_buf: BytesMut::with_capacity(READ_BUF_RESERVE_BYTES),
            decoder: PacketDecoder::default(),
            encoder: PacketEncoder::new(),
            compression_threshold,
            read_timeout: Some(DEFAULT_READ_TIMEOUT),
            last_inbound_at: WallMark::now(),
            login_hello_sent: false,
            serverbound_carried_hotbar_slot: None,
            in_play_phase: false,
            serverbound_play_ids: BTreeSet::new(),
            pending_flush: false,
        }
    }

    pub fn set_read_timeout(&mut self, timeout: Option<Duration>) {
        self.read_timeout = timeout;
        self.last_inbound_at = WallMark::now();
    }

    pub fn compression_threshold(&self) -> Option<i32> {
        self.compression_threshold
    }

    /// Records that the session finished configuration, so every later send is
    /// a play-phase packet.
    ///
    /// Wire ids are per-phase, so the transport cannot tell a play id from a
    /// configuration one on its own; the state machine that made the
    /// transition says so once.
    pub fn enter_play_phase(&mut self) {
        self.in_play_phase = true;
    }

    /// Records that the session left play (e.g. re-entering configuration) so
    /// subsequent configuration packets are not recorded as play-phase ids.
    pub fn exit_play_phase(&mut self) {
        self.in_play_phase = false;
    }

    /// Every distinct play-phase serverbound id this connection put on the
    /// wire, in id order.
    ///
    /// The transport is the owner because it is the one place a packet cannot
    /// bypass: a helper that encodes its own packet and hands it to
    /// `send_packet` is still counted, so this is an observation of the
    /// session rather than a second bookkeeping table beside the senders.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn serverbound_play_ids(&self) -> Vec<i32> {
        self.serverbound_play_ids.iter().copied().collect()
    }

    pub(crate) fn serverbound_carried_hotbar_slot(&self) -> Option<u8> {
        self.serverbound_carried_hotbar_slot
    }

    pub(crate) fn remember_serverbound_carried_hotbar_slot(&mut self, slot: u8) {
        self.serverbound_carried_hotbar_slot = Some(slot.min(8));
    }

    pub fn mark_login_hello_sent(&mut self) -> Result<()> {
        if self.login_hello_sent {
            return Err(anyhow!("login hello was already sent on this connection"));
        }
        self.login_hello_sent = true;
        Ok(())
    }

    pub fn login_hello_sent(&self) -> bool {
        self.login_hello_sent
    }

    pub async fn send_packet(&mut self, packet_id: i32, payload: &[u8]) -> Result<()> {
        if self.in_play_phase {
            self.serverbound_play_ids.insert(packet_id);
        }
        let packet = self
            .encoder
            .encode(packet_id, payload, self.compression_threshold)?;
        self.stream.write_all(packet).await?;
        self.pending_flush = true;
        bbb_client_clock::replay::count(&bbb_client_clock::replay::NET_FRAMES_WRITTEN);
        Ok(())
    }

    pub async fn flush(&mut self) -> Result<()> {
        if self.pending_flush {
            self.stream.flush().await?;
            self.pending_flush = false;
        }
        Ok(())
    }

    pub fn set_compression_threshold(&mut self, threshold: i32) {
        self.compression_threshold = (threshold >= 0).then_some(threshold);
    }

    pub async fn read_packet(&mut self) -> Result<(i32, Vec<u8>)> {
        self.read_packet_or_eof()
            .await?
            .ok_or_else(|| anyhow!("connection closed"))
    }

    /// Reads one packet, answering `None` when the peer closed the connection.
    ///
    /// A session the server ends on purpose is a result rather than a
    /// transport failure, so the close has to be distinguishable from a decode
    /// or socket error instead of collapsing into one `Err`.
    pub async fn read_packet_or_eof(&mut self) -> Result<Option<(i32, Vec<u8>)>> {
        Ok(self
            .read_packet_id_or_eof()
            .await?
            .map(|id| (id, self.payload().to_vec())))
    }

    pub(crate) fn read_timeout_limit(&self) -> Option<Duration> {
        self.read_timeout
            .filter(|_| !bbb_client_clock::is_virtual())
    }

    pub(crate) fn remaining_read_timeout(&self, limit: Duration) -> Duration {
        limit.saturating_sub(self.last_inbound_at.elapsed())
    }

    pub(crate) fn touch_inbound_deadline(&mut self) {
        self.last_inbound_at = WallMark::now();
    }

    pub(crate) fn read_timeout_error(limit: Duration) -> anyhow::Error {
        anyhow!(
            "connection read timed out after {:.1}s",
            limit.as_secs_f64()
        )
    }

    pub(crate) fn try_read_buffered_packet_touching_inbound(&mut self) -> Result<Option<i32>> {
        use bbb_client_clock::replay;
        replay::set(
            &replay::NET_FRAMES_READ,
            replay::NET_FRAMES_HANDED_OUT.load(std::sync::atomic::Ordering::Acquire),
        );
        if let Some(packet) = self.try_read_buffered_packet()? {
            self.last_inbound_at = WallMark::now();
            return Ok(Some(packet));
        }
        Ok(None)
    }

    pub(crate) async fn read_packet_id_or_eof_untimed(&mut self) -> Result<Option<i32>> {
        loop {
            if let Some(packet) = self.try_read_buffered_packet_touching_inbound()? {
                return Ok(Some(packet));
            }

            self.flush().await?;
            self.read_buf.reserve(READ_BUF_RESERVE_BYTES);
            let read = self.stream.read_buf(&mut self.read_buf).await?;
            if read == 0 {
                return Ok(None);
            }
            self.last_inbound_at = WallMark::now();
        }
    }

    pub(crate) async fn read_packet_id_untimed(&mut self) -> Result<i32> {
        self.read_packet_id_or_eof_untimed()
            .await?
            .ok_or_else(|| anyhow!("connection closed"))
    }

    async fn read_packet_id_or_eof(&mut self) -> Result<Option<i32>> {
        loop {
            if let Some(packet) = self.try_read_buffered_packet_touching_inbound()? {
                return Ok(Some(packet));
            }

            self.flush().await?;
            self.read_buf.reserve(READ_BUF_RESERVE_BYTES);
            let read = self.read_socket_chunk_with_timeout().await?;
            if read == 0 {
                return Ok(None);
            }
            self.last_inbound_at = WallMark::now();
        }
    }

    async fn read_socket_chunk_with_timeout(&mut self) -> Result<usize> {
        let Some(limit) = self.read_timeout_limit() else {
            return Ok(self.stream.read_buf(&mut self.read_buf).await?);
        };
        let remaining = self.remaining_read_timeout(limit);
        if remaining.is_zero() {
            return Err(Self::read_timeout_error(limit));
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            match tokio::time::timeout(remaining, self.stream.read_buf(&mut self.read_buf)).await {
                Ok(result) => Ok(result?),
                Err(_) => Err(Self::read_timeout_error(limit)),
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            tokio::select! {
                result = self.stream.read_buf(&mut self.read_buf) => Ok(result?),
                _ = crate::transport::sleep_duration(remaining) => Err(Self::read_timeout_error(limit)),
            }
        }
    }

    /// Decodes one packet that the transport has already read without waiting
    /// for more socket input.
    pub(crate) fn try_read_buffered_packet(&mut self) -> Result<Option<i32>> {
        let id = self
            .decoder
            .try_read(&mut self.read_buf, self.compression_threshold)?;
        if id.is_some() {
            bbb_client_clock::replay::count(&bbb_client_clock::replay::NET_FRAMES_HANDED_OUT);
        }
        Ok(id)
    }

    pub(crate) fn payload(&self) -> &[u8] {
        self.decoder.payload()
    }

    /// Gracefully shuts down and drops the underlying transport stream,
    /// replacing it with a closed sentinel so callers (such as server
    /// transfer) release the old socket before dialing a replacement.
    pub async fn shutdown(&mut self) {
        let mut old = std::mem::replace(&mut self.stream, Box::new(ClosedClientStream));
        let _ = old.shutdown().await;
        drop(old);
    }
}

struct ClosedClientStream;

impl tokio::io::AsyncRead for ClosedClientStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        _buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }
}

impl tokio::io::AsyncWrite for ClosedClientStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        _buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        std::task::Poll::Ready(Err(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "connection closed",
        )))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use tokio::net::{TcpListener, TcpStream};

    #[tokio::test]
    async fn packets_round_trip_over_the_plaintext_stream() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let client_stream = TcpStream::connect(address).await.unwrap();
        let (server_stream, _) = listener.accept().await.unwrap();
        let mut client = RawConnection::from_stream(client_stream, None);
        let mut server = RawConnection::from_stream(server_stream, None);

        client.send_packet(1, b"login-hello").await.unwrap();
        client.send_packet(2, b"second frame").await.unwrap();
        assert_eq!(
            server.read_packet().await.unwrap(),
            (1, b"login-hello".to_vec())
        );
        assert_eq!(
            server.read_packet().await.unwrap(),
            (2, b"second frame".to_vec())
        );

        server.send_packet(4, b"server frame").await.unwrap();
        assert_eq!(
            client.read_packet().await.unwrap(),
            (4, b"server frame".to_vec())
        );
    }

    #[tokio::test]
    async fn new_raw_connection_resets_attack_entity_carried_slot_observation() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();

        let first_stream = TcpStream::connect(address).await.unwrap();
        let (_server_stream, _) = listener.accept().await.unwrap();
        let mut first = RawConnection::from_stream(first_stream, None);
        assert_eq!(first.serverbound_carried_hotbar_slot(), None);
        first.remember_serverbound_carried_hotbar_slot(5);
        assert_eq!(first.serverbound_carried_hotbar_slot(), Some(5));

        let second_stream = TcpStream::connect(address).await.unwrap();
        let (_server_stream, _) = listener.accept().await.unwrap();
        let second = RawConnection::from_stream(second_stream, None);
        assert_eq!(second.serverbound_carried_hotbar_slot(), None);
    }

    #[tokio::test]
    async fn only_the_sends_that_follow_the_play_transition_are_recorded() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let client_stream = TcpStream::connect(address).await.unwrap();
        let (_server_stream, _) = listener.accept().await.unwrap();
        let mut client = RawConnection::from_stream(client_stream, None);

        // Wire ids repeat across phases, so a configuration send must not be
        // reported as the play packet that happens to share its id.
        client.send_packet(7, b"configuration").await.unwrap();
        assert_eq!(client.serverbound_play_ids(), Vec::<i32>::new());

        client.enter_play_phase();
        client.send_packet(9, b"chat").await.unwrap();
        client.send_packet(7, b"chat-command").await.unwrap();
        client.send_packet(9, b"chat again").await.unwrap();
        assert_eq!(client.serverbound_play_ids(), vec![7, 9]);

        client.exit_play_phase();
        client.send_packet(12, b"reconfig").await.unwrap();
        assert_eq!(client.serverbound_play_ids(), vec![7, 9]);
    }

    #[tokio::test]
    async fn read_packet_times_out_when_peer_stalls() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let client_stream = TcpStream::connect(address).await.unwrap();
        let (_server_stream, _) = listener.accept().await.unwrap();
        let mut client = RawConnection::from_stream(client_stream, None);
        client.set_read_timeout(Some(Duration::from_millis(60)));

        let err = client
            .read_packet()
            .await
            .expect_err("silent peer must trigger post-connect read timeout");
        assert!(err.to_string().contains("timed out"), "{err:#}");
    }
}
