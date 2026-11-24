use bbb_protocol::frame::{PacketDecoder, PacketEncoder};
use bytes::BytesMut;
use std::{io, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::timeout,
};

use crate::{session::OutboundPacket, Result};

const WRITE_TIMEOUT: Duration = Duration::from_secs(10);
const INITIAL_READ_BUFFER_CAPACITY: usize = 8 * 1024;
const MAX_IDLE_READ_BUFFER_CAPACITY: usize = 256 * 1024;
pub(crate) const COALESCE_WRITE_BYTES: usize = 64 * 1024;

pub(crate) struct RawTransport {
    stream: TcpStream,
    read_buffer: BytesMut,
    write_buffer: Vec<u8>,
    decoder: PacketDecoder,
    encoder: PacketEncoder,
    compression_threshold: Option<i32>,
}

impl RawTransport {
    pub(crate) fn new(stream: TcpStream) -> Self {
        let _ = stream.set_nodelay(true);
        Self {
            stream,
            read_buffer: BytesMut::with_capacity(INITIAL_READ_BUFFER_CAPACITY),
            write_buffer: Vec::new(),
            decoder: PacketDecoder::default(),
            encoder: PacketEncoder::new(),
            compression_threshold: None,
        }
    }

    pub(crate) async fn read_packet(&mut self) -> Result<Option<(i32, &[u8])>> {
        loop {
            // `CompressionDecoder.java:31-34`: serverbound compressed frames
            // must declare an uncompressed size at or above `threshold`.
            if let Some(packet_id) = self.decoder.try_read_validated(
                &mut self.read_buffer,
                self.compression_threshold,
                true,
            )? {
                if self.read_buffer.is_empty()
                    && self.read_buffer.capacity() > MAX_IDLE_READ_BUFFER_CAPACITY
                {
                    self.read_buffer = BytesMut::with_capacity(INITIAL_READ_BUFFER_CAPACITY);
                }
                return Ok(Some((packet_id, self.decoder.payload())));
            }

            if self.read_buffer.capacity() - self.read_buffer.len() < 4096 {
                self.read_buffer.reserve(4096);
            }
            let read = self.stream.read_buf(&mut self.read_buffer).await?;
            if read == 0 {
                return Ok(None);
            }
        }
    }

    pub(crate) async fn send(&mut self, packet: OutboundPacket) -> Result<()> {
        self.flush_writes().await?;
        let frame = self.encoder.encode(
            packet.packet_id,
            &packet.payload,
            self.compression_threshold,
        )?;
        Self::write_frame(&mut self.stream, frame).await?;
        if let Some(threshold) = packet.compression_after {
            self.compression_threshold = Some(threshold);
        }
        Ok(())
    }

    pub(crate) async fn buffer_or_send_play_packet(
        &mut self,
        packet_id: i32,
        payload: &[u8],
    ) -> Result<usize> {
        let frame = self
            .encoder
            .encode(packet_id, payload, self.compression_threshold)?;
        let len = frame.len();
        if self.write_buffer.is_empty() && len >= COALESCE_WRITE_BYTES {
            Self::write_frame(&mut self.stream, frame).await?;
        } else {
            if self.write_buffer.capacity() == 0 {
                self.write_buffer.reserve(COALESCE_WRITE_BYTES);
            }
            self.write_buffer.extend_from_slice(frame);
        }
        Ok(len)
    }

    pub(crate) async fn buffer_or_send_prepared_play_packet(
        &mut self,
        prepared: &crate::driver::PreparedPlayPacket,
    ) -> Result<usize> {
        if self.compression_threshold == prepared.compression_threshold() {
            let frame = prepared.framed();
            let len = frame.len();
            if self.write_buffer.is_empty() && len >= COALESCE_WRITE_BYTES {
                Self::write_frame(&mut self.stream, frame).await?;
            } else {
                if self.write_buffer.capacity() == 0 {
                    self.write_buffer.reserve(COALESCE_WRITE_BYTES);
                }
                self.write_buffer.extend_from_slice(frame);
            }
            Ok(len)
        } else {
            let mut buffer = BytesMut::from(prepared.framed());
            let mut decoder = PacketDecoder::default();
            let packet_id = decoder
                .try_read(&mut buffer, prepared.compression_threshold())?
                .ok_or_else(|| {
                    bbb_protocol::codec::ProtocolError::InvalidData(
                        "prepared packet did not contain a complete wire frame".to_string(),
                    )
                })?;
            self.buffer_or_send_play_packet(packet_id, decoder.payload())
                .await
        }
    }

    pub(crate) async fn flush_writes(&mut self) -> Result<()> {
        if self.write_buffer.is_empty() {
            return Ok(());
        }
        let result = Self::write_frame(&mut self.stream, &self.write_buffer).await;
        self.write_buffer.clear();
        if self.write_buffer.capacity() > MAX_IDLE_READ_BUFFER_CAPACITY {
            self.write_buffer = Vec::with_capacity(COALESCE_WRITE_BYTES);
        }
        result
    }

    async fn write_frame(stream: &mut TcpStream, frame: &[u8]) -> Result<()> {
        timeout(WRITE_TIMEOUT, stream.write_all(frame))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "connection write timed out"))??;
        Ok(())
    }

    pub(crate) async fn shutdown(&mut self) {
        self.write_buffer.clear();
        let _ = self.stream.shutdown().await;
    }

    #[cfg(test)]
    pub(crate) fn set_compression_threshold_for_test(&mut self, threshold: Option<i32>) {
        self.compression_threshold = threshold;
    }
}
