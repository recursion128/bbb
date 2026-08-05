use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use bbb_protocol::frame::{
    decode_packet_body, encode_packet_with_compression, split_packet, try_read_frame,
};
use bytes::BytesMut;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::{interval, Interval, MissedTickBehavior},
};

pub(crate) fn play_tick_interval() -> Interval {
    let mut tick = interval(Duration::from_millis(50));
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    tick
}

/// Plaintext-only transport: online mode is a declared non-goal, so the vanilla
/// AES/CFB8 stream ciphers are intentionally absent and a login encryption
/// request fails the connection before any cipher could be negotiated.
pub(crate) struct RawConnection {
    pub(crate) stream: TcpStream,
    pub(crate) read_buf: BytesMut,
    pub(crate) compression_threshold: Option<i32>,
    login_hello_sent: bool,
    serverbound_carried_hotbar_slot: Option<u8>,
}

impl RawConnection {
    pub(crate) async fn connect(address: &str, compression_threshold: Option<i32>) -> Result<Self> {
        let stream = TcpStream::connect(address)
            .await
            .with_context(|| format!("connect {address}"))?;
        stream.set_nodelay(true).ok();
        Ok(Self::from_stream(stream, compression_threshold))
    }

    /// Creates the transport owner around an already-connected stream. Production connects and
    /// loopback tests share this path so compression state cannot be omitted by a direct struct
    /// literal.
    pub(crate) fn from_stream(stream: TcpStream, compression_threshold: Option<i32>) -> Self {
        Self {
            stream,
            read_buf: BytesMut::with_capacity(8192),
            compression_threshold,
            login_hello_sent: false,
            serverbound_carried_hotbar_slot: None,
        }
    }

    pub(crate) fn serverbound_carried_hotbar_slot(&self) -> Option<u8> {
        self.serverbound_carried_hotbar_slot
    }

    pub(crate) fn remember_serverbound_carried_hotbar_slot(&mut self, slot: u8) {
        self.serverbound_carried_hotbar_slot = Some(slot.min(8));
    }

    pub(crate) fn mark_login_hello_sent(&mut self) -> Result<()> {
        if self.login_hello_sent {
            return Err(anyhow!("login hello was already sent on this connection"));
        }
        self.login_hello_sent = true;
        Ok(())
    }

    pub(crate) fn login_hello_sent(&self) -> bool {
        self.login_hello_sent
    }

    pub(crate) async fn send_packet(&mut self, packet_id: i32, payload: &[u8]) -> Result<()> {
        let packet =
            encode_packet_with_compression(packet_id, payload, self.compression_threshold)?;
        self.stream.write_all(&packet).await?;
        Ok(())
    }

    pub(crate) fn set_compression_threshold(&mut self, threshold: i32) {
        self.compression_threshold = (threshold >= 0).then_some(threshold);
    }

    pub(crate) async fn read_packet(&mut self) -> Result<(i32, Vec<u8>)> {
        loop {
            if let Some(packet) = self.try_read_buffered_packet()? {
                return Ok(packet);
            }

            let mut temp = [0u8; 4096];
            let read = self.stream.read(&mut temp).await?;
            if read == 0 {
                return Err(anyhow!("connection closed"));
            }
            self.read_buf.extend_from_slice(&temp[..read]);
        }
    }

    /// Decodes one packet that the transport has already read without waiting
    /// for more socket input.
    pub(crate) fn try_read_buffered_packet(&mut self) -> Result<Option<(i32, Vec<u8>)>> {
        let Some(frame) = try_read_frame(&mut self.read_buf)? else {
            return Ok(None);
        };
        let body = decode_packet_body(&frame, self.compression_threshold)?;
        let (packet_id, payload) = split_packet(&body)?;
        Ok(Some((packet_id, payload.to_vec())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

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
}
