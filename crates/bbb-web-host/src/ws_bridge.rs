//! Bidirectional binary WebSocket <-> Minecraft TCP stream relay with target allowlist.

use std::{collections::HashSet, time::Duration};

use anyhow::Result;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::{
    http::{HttpRequest, HttpResponse},
    ws_codec::{complete_ws_upgrade, validate_ws_upgrade, WsMessage},
};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const RELAY_BATCH_CAP: usize = 64 * 1024;
const PONG_QUEUE_CAP: usize = 64;

pub async fn handle_ws_relay(
    req: HttpRequest,
    mut client_stream: TcpStream,
    leftover: Vec<u8>,
    default_server: &str,
    allowed_servers: &HashSet<String>,
) -> Result<()> {
    let requested_target = req
        .query_param("server")
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| default_server.to_string());

    let Some(target) = resolve_allowed_target(&requested_target, default_server, allowed_servers)
    else {
        HttpResponse::text(
            403,
            "Forbidden",
            format!(
                "Upstream server '{requested_target}' is not permitted by this bbb-web-host instance"
            ),
        )
        .write_to(&mut client_stream, false)
        .await?;
        return Ok(());
    };

    let accept_key = match validate_ws_upgrade(&req) {
        Ok(key) => key,
        Err(resp) => {
            resp.write_to(&mut client_stream, false).await?;
            return Ok(());
        }
    };

    let upstream = match tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect(&target)).await {
        Ok(Ok(stream)) => stream,
        Ok(Err(err)) => {
            HttpResponse::text(
                502,
                "Bad Gateway",
                format!("Failed to connect upstream {target}: {err}"),
            )
            .write_to(&mut client_stream, false)
            .await?;
            return Ok(());
        }
        Err(_) => {
            HttpResponse::text(
                504,
                "Gateway Timeout",
                format!("Timed out connecting upstream {target}"),
            )
            .write_to(&mut client_stream, false)
            .await?;
            return Ok(());
        }
    };
    let _ = upstream.set_nodelay(true);
    let _ = client_stream.set_nodelay(true);

    let ws = complete_ws_upgrade(&accept_key, client_stream, leftover).await?;

    let (mut ws_reader, mut ws_writer) = ws.into_parts();
    let (mut tcp_reader, mut tcp_writer) = upstream.into_split();
    let (pong_tx, mut pong_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(PONG_QUEUE_CAP);

    let ws_to_tcp = async move {
        let mut tcp_batch = Vec::with_capacity(RELAY_BATCH_CAP);
        'outer: while let Some(first_msg) = ws_reader.recv().await? {
            let mut cur = Some(first_msg);
            let mut should_close = false;
            while let Some(msg) = cur {
                match msg {
                    WsMessage::Binary(bytes) => {
                        if !bytes.is_empty() {
                            if !tcp_batch.is_empty()
                                && tcp_batch.len() + bytes.len() > RELAY_BATCH_CAP
                            {
                                tcp_writer.write_all(&tcp_batch).await?;
                                tcp_batch.clear();
                            }
                            if bytes.len() > RELAY_BATCH_CAP {
                                tcp_writer.write_all(&bytes).await?;
                            } else {
                                tcp_batch.extend_from_slice(&bytes);
                            }
                        }
                    }
                    WsMessage::Text(text) => {
                        let bytes = text.as_bytes();
                        if !bytes.is_empty() {
                            if !tcp_batch.is_empty()
                                && tcp_batch.len() + bytes.len() > RELAY_BATCH_CAP
                            {
                                tcp_writer.write_all(&tcp_batch).await?;
                                tcp_batch.clear();
                            }
                            if bytes.len() > RELAY_BATCH_CAP {
                                tcp_writer.write_all(bytes).await?;
                            } else {
                                tcp_batch.extend_from_slice(bytes);
                            }
                        }
                    }
                    WsMessage::Ping(payload) => {
                        let _ = pong_tx.try_send(payload);
                    }
                    WsMessage::Pong(_) => {}
                    WsMessage::Close => {
                        should_close = true;
                        break;
                    }
                }
                cur = ws_reader.try_recv_buffered()?;
            }
            if !tcp_batch.is_empty() {
                tcp_writer.write_all(&tcp_batch).await?;
                tcp_batch.clear();
            }
            tcp_writer.flush().await?;
            if should_close {
                break 'outer;
            }
        }
        let _ = tcp_writer.shutdown().await;
        Ok::<(), anyhow::Error>(())
    };

    let tcp_to_ws = async move {
        let mut buf = vec![0u8; RELAY_BATCH_CAP];
        loop {
            tokio::select! {
                Some(pong_payload) = pong_rx.recv() => {
                    ws_writer.queue_message(&WsMessage::Pong(pong_payload)).await?;
                    while let Ok(more_pong) = pong_rx.try_recv() {
                        ws_writer.queue_message(&WsMessage::Pong(more_pong)).await?;
                    }
                    ws_writer.flush().await?;
                }
                read_res = tcp_reader.read(&mut buf) => {
                    let n = read_res?;
                    if n == 0 {
                        let _ = ws_writer.send(&WsMessage::Close).await;
                        break;
                    }
                    ws_writer.queue_binary(&buf[..n]).await?;
                    while let Ok(pong_payload) = pong_rx.try_recv() {
                        ws_writer.queue_message(&WsMessage::Pong(pong_payload)).await?;
                    }
                    ws_writer.flush().await?;
                }
            }
        }
        Ok::<(), anyhow::Error>(())
    };

    tokio::select! {
        res = ws_to_tcp => {
            if let Err(err) = res {
                tracing::debug!(?err, %target, "ws->tcp relay ended");
            }
        }
        res = tcp_to_ws => {
            if let Err(err) = res {
                tracing::debug!(?err, %target, "tcp->ws relay ended");
            }
        }
    }

    Ok(())
}

fn parse_loopback_port(addr: &str) -> Option<u16> {
    let (host, port_str) = addr.trim().rsplit_once(':')?;
    let port: u16 = port_str.parse().ok()?;
    matches!(
        host.to_ascii_lowercase().as_str(),
        "127.0.0.1" | "localhost" | "[::1]"
    )
    .then_some(port)
}

fn resolve_allowed_target(
    candidate: &str,
    default_server: &str,
    allowed_servers: &HashSet<String>,
) -> Option<String> {
    let trimmed = candidate.trim();
    let normalized = trimmed.to_ascii_lowercase();
    if normalized == default_server.trim().to_ascii_lowercase()
        || allowed_servers.contains(&normalized)
    {
        return Some(trimmed.to_string());
    }
    let cand_port = parse_loopback_port(trimmed)?;
    if parse_loopback_port(default_server) == Some(cand_port) {
        return Some(default_server.trim().to_string());
    }
    allowed_servers
        .iter()
        .find(|allowed| parse_loopback_port(allowed) == Some(cand_port))
        .cloned()
}
