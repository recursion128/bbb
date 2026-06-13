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

pub(crate) struct RawConnection {
    pub(crate) stream: TcpStream,
    pub(crate) read_buf: BytesMut,
    pub(crate) compression_threshold: Option<i32>,
}

impl RawConnection {
    pub(crate) async fn connect(address: &str, compression_threshold: Option<i32>) -> Result<Self> {
        let stream = TcpStream::connect(address)
            .await
            .with_context(|| format!("connect {address}"))?;
        stream.set_nodelay(true).ok();
        Ok(Self {
            stream,
            read_buf: BytesMut::with_capacity(8192),
            compression_threshold,
        })
    }

    pub(crate) async fn send_packet(&mut self, packet_id: i32, payload: &[u8]) -> Result<()> {
        let packet =
            encode_packet_with_compression(packet_id, payload, self.compression_threshold)?;
        self.stream.write_all(&packet).await?;
        Ok(())
    }

    pub(crate) async fn read_packet(&mut self) -> Result<(i32, Vec<u8>)> {
        loop {
            if let Some(frame) = try_read_frame(&mut self.read_buf)? {
                let body = decode_packet_body(&frame, self.compression_threshold)?;
                let (packet_id, payload) = split_packet(&body)?;
                return Ok((packet_id, payload.to_vec()));
            }

            let mut temp = [0u8; 4096];
            let read = self.stream.read(&mut temp).await?;
            if read == 0 {
                return Err(anyhow!("connection closed"));
            }
            self.read_buf.extend_from_slice(&temp[..read]);
        }
    }
}
