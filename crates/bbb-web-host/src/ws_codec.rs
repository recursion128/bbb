//! RFC 6455 WebSocket HTTP upgrade handshake and buffered async frame codec.

use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use sha1::{Digest, Sha1};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::http::{HttpMethod, HttpRequest, HttpResponse};

const WS_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
const MAX_WS_MESSAGE_BYTES: usize = 64 * 1024 * 1024;
const MAX_WS_CONTROL_PAYLOAD_BYTES: usize = 125;
const WS_READ_BUF_INITIAL: usize = 64 * 1024;
const WS_READ_GROW_STEP: usize = 1024 * 1024;
const WS_WRITE_FLUSH_THRESHOLD: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WsMessage {
    Text(String),
    Binary(Vec<u8>),
    Ping(Vec<u8>),
    Pong(Vec<u8>),
    Close,
}

pub fn compute_accept_key(sec_websocket_key: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(sec_websocket_key.trim().as_bytes());
    hasher.update(WS_GUID.as_bytes());
    BASE64.encode(hasher.finalize())
}

pub fn validate_ws_upgrade(req: &HttpRequest) -> std::result::Result<String, HttpResponse> {
    let is_upgrade = req
        .header("upgrade")
        .is_some_and(|v| v.eq_ignore_ascii_case("websocket"));
    let Some(key) = req
        .header("sec-websocket-key")
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Err(HttpResponse::text(
            400,
            "Bad Request",
            "Missing Sec-WebSocket-Key",
        ));
    };
    if req.method != HttpMethod::Get || !is_upgrade {
        return Err(HttpResponse::text(
            400,
            "Bad Request",
            "Expected WebSocket GET upgrade",
        ));
    }
    Ok(compute_accept_key(key))
}

pub async fn complete_ws_upgrade<S>(
    accept_key: &str,
    mut stream: S,
    leftover: Vec<u8>,
) -> Result<WsStream<S>>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let handshake = format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: {accept_key}\r\n\
         Cross-Origin-Resource-Policy: same-origin\r\n\r\n"
    );
    stream
        .write_all(handshake.as_bytes())
        .await
        .context("write WebSocket 101 upgrade")?;
    stream
        .flush()
        .await
        .context("flush WebSocket 101 upgrade")?;

    Ok(WsStream::new(stream, leftover))
}

pub async fn upgrade_into_ws<S>(
    req: &HttpRequest,
    mut stream: S,
    leftover: Vec<u8>,
) -> Result<Option<WsStream<S>>>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let accept = match validate_ws_upgrade(req) {
        Ok(accept) => accept,
        Err(resp) => {
            resp.write_to(&mut stream, false).await?;
            return Ok(None);
        }
    };
    Ok(Some(complete_ws_upgrade(&accept, stream, leftover).await?))
}

struct WsReadState {
    buf: Vec<u8>,
    pos: usize,
    filled: usize,
    frag_opcode: Option<u8>,
    frag_payload: Vec<u8>,
}

impl WsReadState {
    fn from_leftover(mut leftover: Vec<u8>) -> Self {
        let filled = leftover.len();
        let target_len = filled.max(WS_READ_BUF_INITIAL);
        leftover.resize(target_len, 0);
        Self {
            buf: leftover,
            pos: 0,
            filled,
            frag_opcode: None,
            frag_payload: Vec::new(),
        }
    }

    fn peek_frame_requirement(&self) -> Result<Option<(usize, usize)>> {
        let avail = &self.buf[self.pos..self.filled];
        if avail.len() < 2 {
            return Ok(None);
        }
        let b0 = avail[0];
        let fin = (b0 & 0x80) != 0;
        let opcode = b0 & 0x0f;
        let is_control = (opcode & 0x08) != 0;
        let masked = (avail[1] & 0x80) != 0;
        let len_code = (avail[1] & 0x7f) as usize;
        if is_control {
            if !fin {
                bail!("fragmented WebSocket control frame (opcode {opcode:#x})");
            }
            if len_code > MAX_WS_CONTROL_PAYLOAD_BYTES {
                bail!(
                    "WebSocket control frame length code {len_code} exceeds RFC 6455 limit {MAX_WS_CONTROL_PAYLOAD_BYTES}"
                );
            }
        }
        let (ext_bytes, payload_len) = match len_code {
            0..=125 => (0usize, len_code),
            126 => {
                if avail.len() < 4 {
                    return Ok(None);
                }
                (2usize, u16::from_be_bytes([avail[2], avail[3]]) as usize)
            }
            127 => {
                if avail.len() < 10 {
                    return Ok(None);
                }
                let raw_len = u64::from_be_bytes([
                    avail[2], avail[3], avail[4], avail[5], avail[6], avail[7], avail[8], avail[9],
                ]);
                let parsed = usize::try_from(raw_len).context("WebSocket frame too large")?;
                (8usize, parsed)
            }
            _ => unreachable!(),
        };
        if payload_len > MAX_WS_MESSAGE_BYTES {
            bail!("WebSocket frame length {payload_len} exceeds limit {MAX_WS_MESSAGE_BYTES}");
        }
        let header_total = 2 + ext_bytes + if masked { 4 } else { 0 };
        Ok(Some((header_total, payload_len)))
    }

    fn try_parse_buffered_message(&mut self) -> Result<Option<WsMessage>> {
        loop {
            let Some((header_total, payload_len)) = self.peek_frame_requirement()? else {
                return Ok(None);
            };
            let frame_total = header_total + payload_len;
            let avail_len = self.filled - self.pos;
            if avail_len < frame_total {
                return Ok(None);
            }

            let frame_start = self.pos;
            self.pos += frame_total;
            let b0 = self.buf[frame_start];
            let b1 = self.buf[frame_start + 1];
            let fin = (b0 & 0x80) != 0;
            let opcode = b0 & 0x0f;
            let masked = (b1 & 0x80) != 0;
            let payload_start = frame_start + header_total;
            let payload_end = payload_start + payload_len;

            if masked {
                let mask_pos = payload_start - 4;
                let key = [
                    self.buf[mask_pos],
                    self.buf[mask_pos + 1],
                    self.buf[mask_pos + 2],
                    self.buf[mask_pos + 3],
                ];
                xor_mask_in_place(&mut self.buf[payload_start..payload_end], key);
            }

            let reset_after = self.pos == self.filled;
            let result = match opcode {
                0x8 => Some(WsMessage::Close),
                0x9 => Some(WsMessage::Ping(
                    self.buf[payload_start..payload_end].to_vec(),
                )),
                0xA => Some(WsMessage::Pong(
                    self.buf[payload_start..payload_end].to_vec(),
                )),
                0x1 | 0x2 => {
                    if self.frag_opcode.is_some() {
                        bail!(
                            "received new WebSocket data frame while fragmented message in flight"
                        );
                    }
                    let slice = &self.buf[payload_start..payload_end];
                    if fin {
                        Some(decode_complete_slice(opcode, slice)?)
                    } else {
                        self.frag_opcode = Some(opcode);
                        self.frag_payload.clear();
                        self.frag_payload.extend_from_slice(slice);
                        None
                    }
                }
                0x0 => {
                    let Some(active_opcode) = self.frag_opcode else {
                        bail!("unexpected WebSocket continuation frame");
                    };
                    let slice = &self.buf[payload_start..payload_end];
                    if self.frag_payload.len().saturating_add(slice.len()) > MAX_WS_MESSAGE_BYTES {
                        bail!("fragmented WebSocket message exceeded {MAX_WS_MESSAGE_BYTES} bytes");
                    }
                    self.frag_payload.extend_from_slice(slice);
                    if fin {
                        self.frag_opcode = None;
                        let full = std::mem::take(&mut self.frag_payload);
                        Some(decode_complete_vec(active_opcode, full)?)
                    } else {
                        None
                    }
                }
                other => bail!("unrecognized WebSocket opcode {other:#x}"),
            };

            if reset_after {
                self.pos = 0;
                self.filled = 0;
                if self.buf.len() > WS_READ_BUF_INITIAL * 4 {
                    self.buf.truncate(WS_READ_BUF_INITIAL);
                    self.buf.shrink_to(WS_READ_BUF_INITIAL);
                }
            }

            if let Some(msg) = result {
                return Ok(Some(msg));
            }
        }
    }

    async fn recv_message<R>(&mut self, inner: &mut R) -> Result<Option<WsMessage>>
    where
        R: AsyncRead + Unpin,
    {
        loop {
            if let Some(msg) = self.try_parse_buffered_message()? {
                return Ok(Some(msg));
            }

            if self.pos > 0 {
                let remaining = self.filled - self.pos;
                if remaining > 0 {
                    self.buf.copy_within(self.pos..self.filled, 0);
                }
                self.pos = 0;
                self.filled = remaining;
            }

            if let Some((header_total, payload_len)) = self.peek_frame_requirement()? {
                let needed = header_total
                    .checked_add(payload_len)
                    .context("WebSocket frame total length overflow")?;
                // Grow incrementally as payload bytes actually arrive so a
                // 10-byte header claiming 64 MiB cannot force an eager 64 MiB allocation.
                if self.filled == self.buf.len() && self.buf.len() < needed {
                    let grow_cap = self.buf.len().saturating_add(WS_READ_GROW_STEP);
                    let next_len = self
                        .buf
                        .len()
                        .saturating_mul(2)
                        .min(grow_cap)
                        .min(needed)
                        .max(self.filled + 1);
                    self.buf
                        .try_reserve(next_len - self.buf.len())
                        .map_err(|_| {
                            anyhow::anyhow!(
                                "failed to grow WebSocket read buffer to {next_len} bytes"
                            )
                        })?;
                    self.buf.resize(next_len, 0);
                }
            } else if self.filled == self.buf.len() {
                let next_len = self
                    .buf
                    .len()
                    .saturating_add(WS_READ_BUF_INITIAL)
                    .min(MAX_WS_MESSAGE_BYTES + 14);
                self.buf.resize(next_len, 0);
            }

            let n = inner
                .read(&mut self.buf[self.filled..])
                .await
                .context("read WebSocket bytes")?;
            if n == 0 {
                if self.filled == 0 && self.frag_opcode.is_none() {
                    return Ok(None);
                }
                bail!("unexpected EOF mid WebSocket frame");
            }
            self.filled += n;
        }
    }
}

pub struct WsStream<S> {
    inner: S,
    read_state: WsReadState,
    write_buf: Vec<u8>,
}

impl<S> WsStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(inner: S, leftover: Vec<u8>) -> Self {
        Self {
            inner,
            read_state: WsReadState::from_leftover(leftover),
            write_buf: Vec::with_capacity(4096),
        }
    }

    pub fn into_parts(
        self,
    ) -> (
        WsReader<tokio::io::ReadHalf<S>>,
        WsWriter<tokio::io::WriteHalf<S>>,
    )
    where
        S: AsyncRead + AsyncWrite,
    {
        let (read_half, write_half) = tokio::io::split(self.inner);
        (
            WsReader {
                inner: read_half,
                state: self.read_state,
            },
            WsWriter {
                inner: write_half,
                write_buf: Vec::with_capacity(WS_WRITE_FLUSH_THRESHOLD),
            },
        )
    }

    pub async fn recv(&mut self) -> Result<Option<WsMessage>> {
        self.read_state.recv_message(&mut self.inner).await
    }

    pub async fn send(&mut self, msg: &WsMessage) -> Result<()> {
        let (opcode, payload) = message_opcode_and_payload(msg);
        queue_frame_bytes(&mut self.inner, &mut self.write_buf, opcode, payload, false).await?;
        flush_write_buf(&mut self.inner, &mut self.write_buf).await
    }
}

pub struct WsReader<R> {
    inner: R,
    state: WsReadState,
}

impl<R> WsReader<R>
where
    R: AsyncRead + Unpin,
{
    pub async fn recv(&mut self) -> Result<Option<WsMessage>> {
        self.state.recv_message(&mut self.inner).await
    }

    pub fn try_recv_buffered(&mut self) -> Result<Option<WsMessage>> {
        self.state.try_parse_buffered_message()
    }
}

pub struct WsWriter<W> {
    inner: W,
    write_buf: Vec<u8>,
}

impl<W> WsWriter<W>
where
    W: AsyncWrite + Unpin,
{
    pub async fn queue_message(&mut self, msg: &WsMessage) -> Result<()> {
        let (opcode, payload) = message_opcode_and_payload(msg);
        queue_frame_bytes(&mut self.inner, &mut self.write_buf, opcode, payload, false).await
    }

    pub async fn queue_binary(&mut self, bytes: &[u8]) -> Result<()> {
        queue_frame_bytes(&mut self.inner, &mut self.write_buf, 0x2, bytes, false).await
    }

    pub async fn flush(&mut self) -> Result<()> {
        flush_write_buf(&mut self.inner, &mut self.write_buf).await
    }

    pub async fn send(&mut self, msg: &WsMessage) -> Result<()> {
        self.queue_message(msg).await?;
        self.flush().await
    }
}

fn xor_mask_in_place(payload: &mut [u8], key: [u8; 4]) {
    let key8 = u64::from_ne_bytes([
        key[0], key[1], key[2], key[3], key[0], key[1], key[2], key[3],
    ]);
    let (chunks, remainder) = payload.as_chunks_mut::<8>();
    for chunk in chunks {
        let word = u64::from_ne_bytes(*chunk);
        chunk.copy_from_slice(&(word ^ key8).to_ne_bytes());
    }
    for (idx, byte) in remainder.iter_mut().enumerate() {
        *byte ^= key[idx & 3];
    }
}

fn decode_complete_slice(opcode: u8, payload: &[u8]) -> Result<WsMessage> {
    match opcode {
        0x1 => Ok(WsMessage::Text(
            std::str::from_utf8(payload)
                .context("WebSocket text frame is not valid UTF-8")?
                .to_owned(),
        )),
        0x2 => Ok(WsMessage::Binary(payload.to_vec())),
        _ => bail!("invalid data opcode {opcode}"),
    }
}

fn decode_complete_vec(opcode: u8, payload: Vec<u8>) -> Result<WsMessage> {
    match opcode {
        0x1 => Ok(WsMessage::Text(
            String::from_utf8(payload).context("WebSocket text frame is not valid UTF-8")?,
        )),
        0x2 => Ok(WsMessage::Binary(payload)),
        _ => bail!("invalid data opcode {opcode}"),
    }
}

fn message_opcode_and_payload(msg: &WsMessage) -> (u8, &[u8]) {
    match msg {
        WsMessage::Text(text) => (0x1, text.as_bytes()),
        WsMessage::Binary(bytes) => (0x2, bytes.as_slice()),
        WsMessage::Close => (0x8, &[]),
        WsMessage::Ping(bytes) => (0x9, bytes.as_slice()),
        WsMessage::Pong(bytes) => (0xA, bytes.as_slice()),
    }
}

fn encode_frame_header(opcode: u8, len: usize, masked: bool) -> ([u8; 14], usize) {
    let mut head = [0u8; 14];
    head[0] = 0x80 | opcode;
    let mask_bit = if masked { 0x80 } else { 0x00 };
    let mut pos = if len <= 125 {
        head[1] = mask_bit | (len as u8);
        2
    } else if len <= u16::MAX as usize {
        head[1] = mask_bit | 126;
        let ext = (len as u16).to_be_bytes();
        head[2] = ext[0];
        head[3] = ext[1];
        4
    } else {
        head[1] = mask_bit | 127;
        let ext = (len as u64).to_be_bytes();
        head[2..10].copy_from_slice(&ext);
        10
    };
    if masked {
        head[pos..pos + 4].copy_from_slice(&[0x37, 0xfa, 0x21, 0x3d]);
        pos += 4;
    }
    (head, pos)
}

async fn queue_frame_bytes<W>(
    inner: &mut W,
    write_buf: &mut Vec<u8>,
    opcode: u8,
    payload: &[u8],
    masked: bool,
) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    let (head, head_len) = encode_frame_header(opcode, payload.len(), masked);
    let total_frame_len = head_len + payload.len();

    if !write_buf.is_empty() && write_buf.len() + total_frame_len > WS_WRITE_FLUSH_THRESHOLD {
        inner
            .write_all(write_buf)
            .await
            .context("write batched WebSocket frames")?;
        write_buf.clear();
    }

    if !masked && total_frame_len > WS_WRITE_FLUSH_THRESHOLD {
        inner
            .write_all(&head[..head_len])
            .await
            .context("write WebSocket header")?;
        inner
            .write_all(payload)
            .await
            .context("write WebSocket payload")?;
        return Ok(());
    }

    write_buf.extend_from_slice(&head[..head_len]);
    let start = write_buf.len();
    write_buf.extend_from_slice(payload);
    if masked {
        xor_mask_in_place(&mut write_buf[start..], [0x37, 0xfa, 0x21, 0x3d]);
    }
    Ok(())
}

async fn flush_write_buf<W>(inner: &mut W, write_buf: &mut Vec<u8>) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    if !write_buf.is_empty() {
        inner
            .write_all(write_buf)
            .await
            .context("write WebSocket frame buffer")?;
        write_buf.clear();
    }
    inner.flush().await.context("flush WebSocket frame")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xor_mask_u64_matches_bytewise_across_lengths_and_offsets() {
        let key = [0x9a, 0x3c, 0x71, 0xe5];
        for len in [0, 1, 2, 3, 4, 7, 8, 9, 15, 16, 31, 63, 64, 129] {
            let mut fast: Vec<u8> = (0..len).map(|i| (i * 37 + 11) as u8).collect();
            let mut expected = fast.clone();
            for (idx, b) in expected.iter_mut().enumerate() {
                *b ^= key[idx & 3];
            }
            xor_mask_in_place(&mut fast, key);
            assert_eq!(fast, expected, "mismatch at len={len}");
        }
    }

    #[tokio::test]
    async fn rejects_fragmented_and_oversized_control_frames() {
        // Fragmented Ping (FIN=0, opcode=0x9, len=0)
        let (mut tx, rx) = tokio::io::duplex(64);
        tx.write_all(&[0x09, 0x00]).await.unwrap();
        drop(tx);
        let mut ws = WsStream::new(rx, Vec::new());
        assert!(ws.recv().await.is_err());

        // Oversized Ping (FIN=1, opcode=0x9, len_code=126, ext_len=126)
        let (mut tx, rx) = tokio::io::duplex(256);
        let mut frame = vec![0x89, 126, 0, 126];
        frame.extend_from_slice(&[0u8; 126]);
        tx.write_all(&frame).await.unwrap();
        drop(tx);
        let mut ws = WsStream::new(rx, Vec::new());
        assert!(ws.recv().await.is_err());
    }

    #[tokio::test]
    async fn does_not_eagerly_allocate_buffer_for_unreceived_huge_frame() {
        let (mut tx, rx) = tokio::io::duplex(64);
        // Binary frame claiming 64 MiB payload, followed immediately by EOF.
        let mut header = vec![0x82, 127];
        header.extend_from_slice(&(MAX_WS_MESSAGE_BYTES as u64).to_be_bytes());
        tx.write_all(&header).await.unwrap();
        drop(tx);

        let mut ws = WsStream::new(rx, Vec::new());
        assert!(ws.recv().await.is_err());
        assert!(
            ws.read_state.buf.len() <= WS_READ_BUF_INITIAL,
            "buffer eagerly grew to {} bytes without receiving payload",
            ws.read_state.buf.len()
        );
    }
}
