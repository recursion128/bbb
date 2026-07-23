use std::time::Duration;

use aes::{
    cipher::{Block, BlockEncrypt, KeyInit},
    Aes128,
};
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

pub(crate) struct RawConnection {
    pub(crate) stream: TcpStream,
    pub(crate) read_buf: BytesMut,
    pub(crate) compression_threshold: Option<i32>,
    decrypt_cipher: Option<MinecraftCfb8>,
    encrypt_cipher: Option<MinecraftCfb8>,
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
    /// loopback tests share this path so compression and encryption state cannot be omitted by a
    /// direct struct literal.
    pub(crate) fn from_stream(stream: TcpStream, compression_threshold: Option<i32>) -> Self {
        Self {
            stream,
            read_buf: BytesMut::with_capacity(8192),
            compression_threshold,
            decrypt_cipher: None,
            encrypt_cipher: None,
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

    /// Installs vanilla's `AES/CFB8/NoPadding` stream ciphers after the plaintext login-key packet
    /// has been fully written. The 128-bit secret is also the IV (`Crypt.getCipher`). Buffered
    /// plaintext is rejected because switching modes with an undecoded prefix would make frame
    /// ownership ambiguous.
    pub(crate) fn enable_encryption(&mut self, secret: [u8; 16]) -> Result<()> {
        if self.encrypt_cipher.is_some() || self.decrypt_cipher.is_some() {
            return Err(anyhow!("connection encryption is already enabled"));
        }
        if !self.read_buf.is_empty() {
            return Err(anyhow!(
                "cannot enable connection encryption with {} buffered plaintext bytes",
                self.read_buf.len()
            ));
        }
        self.encrypt_cipher = Some(MinecraftCfb8::new(secret));
        self.decrypt_cipher = Some(MinecraftCfb8::new(secret));
        Ok(())
    }

    pub(crate) fn encryption_enabled(&self) -> bool {
        self.encrypt_cipher.is_some() && self.decrypt_cipher.is_some()
    }

    pub(crate) async fn send_packet(&mut self, packet_id: i32, payload: &[u8]) -> Result<()> {
        let mut packet =
            encode_packet_with_compression(packet_id, payload, self.compression_threshold)?;
        if let Some(cipher) = &mut self.encrypt_cipher {
            cipher.encrypt(&mut packet);
        }
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
            if let Some(cipher) = &mut self.decrypt_cipher {
                cipher.decrypt(&mut temp[..read]);
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

/// Stateful AES/CFB8 transform used by Minecraft's encrypted TCP stream. Each byte encrypts the
/// current 16-byte feedback register with AES-128 and XORs the first output byte. Encryption shifts
/// in ciphertext output; decryption shifts in ciphertext input. Packet boundaries do not reset it.
struct MinecraftCfb8 {
    cipher: Aes128,
    feedback: [u8; 16],
}

impl MinecraftCfb8 {
    fn new(secret: [u8; 16]) -> Self {
        Self {
            cipher: Aes128::new_from_slice(&secret).expect("AES-128 accepts a 16-byte key"),
            feedback: secret,
        }
    }

    fn encrypt(&mut self, bytes: &mut [u8]) {
        for byte in bytes {
            let key_byte = self.next_key_byte();
            *byte ^= key_byte;
            self.shift_feedback(*byte);
        }
    }

    fn decrypt(&mut self, bytes: &mut [u8]) {
        for byte in bytes {
            let ciphertext = *byte;
            let key_byte = self.next_key_byte();
            *byte ^= key_byte;
            self.shift_feedback(ciphertext);
        }
    }

    fn next_key_byte(&self) -> u8 {
        let mut block = Block::<Aes128>::clone_from_slice(&self.feedback);
        self.cipher.encrypt_block(&mut block);
        block[0]
    }

    fn shift_feedback(&mut self, ciphertext: u8) {
        self.feedback.copy_within(1.., 0);
        self.feedback[15] = ciphertext;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[test]
    fn minecraft_cfb8_matches_aes_cfb8_known_stream_and_preserves_chunk_state() {
        // OpenSSL `aes-128-cfb8` reference for key=IV="0123456789abcdef" and the plaintext below.
        // Splitting the calls proves the feedback register survives arbitrary socket read chunks.
        let secret = *b"0123456789abcdef";
        let plaintext = b"minecraft-cfb8-stream";
        let expected = [
            0x1f, 0xe9, 0x72, 0xd8, 0x3a, 0x01, 0xc8, 0x10, 0xea, 0xcf, 0x77, 0xae, 0xf0, 0xe5,
            0xc1, 0x1d, 0xd1, 0x31, 0x89, 0x18, 0xb1,
        ];

        let mut encrypted = plaintext.to_vec();
        let mut encrypt = MinecraftCfb8::new(secret);
        encrypt.encrypt(&mut encrypted[..7]);
        encrypt.encrypt(&mut encrypted[7..]);
        assert_eq!(encrypted, expected);

        let mut decrypt = MinecraftCfb8::new(secret);
        decrypt.decrypt(&mut encrypted[..3]);
        decrypt.decrypt(&mut encrypted[3..15]);
        decrypt.decrypt(&mut encrypted[15..]);
        assert_eq!(encrypted, plaintext);
    }

    #[tokio::test]
    async fn raw_connection_switches_from_plaintext_key_to_encrypted_packet_stream() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let client_stream = TcpStream::connect(address).await.unwrap();
        let (server_stream, _) = listener.accept().await.unwrap();
        let mut client = RawConnection::from_stream(client_stream, None);
        let mut server = RawConnection::from_stream(server_stream, None);

        // The login-key packet is plaintext. Vanilla enables the ciphers only after its write
        // completion callback, then every subsequent frame byte is encrypted continuously.
        client.send_packet(1, b"login-key").await.unwrap();
        assert_eq!(
            server.read_packet().await.unwrap(),
            (1, b"login-key".to_vec())
        );
        let secret = *b"0123456789abcdef";
        client.enable_encryption(secret).unwrap();
        server.enable_encryption(secret).unwrap();
        assert!(client.encryption_enabled());
        assert!(server.encryption_enabled());

        client.send_packet(2, b"client encrypted").await.unwrap();
        client.send_packet(3, b"continuous state").await.unwrap();
        assert_eq!(
            server.read_packet().await.unwrap(),
            (2, b"client encrypted".to_vec())
        );
        assert_eq!(
            server.read_packet().await.unwrap(),
            (3, b"continuous state".to_vec())
        );

        server.send_packet(4, b"server encrypted").await.unwrap();
        assert_eq!(
            client.read_packet().await.unwrap(),
            (4, b"server encrypted".to_vec())
        );
        assert!(client.enable_encryption(secret).is_err());
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
    async fn encryption_rejects_buffered_plaintext() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let client_stream = TcpStream::connect(address).await.unwrap();
        let (_server_stream, _) = listener.accept().await.unwrap();
        let mut client = RawConnection::from_stream(client_stream, None);
        client.read_buf.extend_from_slice(b"plaintext");

        let error = client.enable_encryption(*b"0123456789abcdef").unwrap_err();
        assert!(error.to_string().contains("buffered plaintext"));
        assert!(!client.encryption_enabled());
    }
}
