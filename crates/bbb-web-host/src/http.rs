//! HTTP/1.1 request head adapter (backed by `bbb_mcp::http`) and cross-origin-isolated
//! keep-alive response writer.

use anyhow::{Context, Result};
use bbb_mcp::http::{read_request, read_request_head, HttpLimits};
pub use bbb_mcp::http::{HttpMethod, HttpRequest};
use bytes::Bytes;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};

const WEB_HOST_HTTP_LIMITS: HttpLimits = HttpLimits {
    max_request_line_bytes: 8 * 1024,
    max_header_bytes: 32 * 1024,
    max_headers: 64,
    max_body_bytes: 0,
};

const SMALL_BODY_COALESCE_BYTES: usize = 16 * 1024;

pub async fn read_http_request<R>(stream: &mut R) -> Result<Option<(HttpRequest, Vec<u8>)>>
where
    R: AsyncRead + Unpin,
{
    read_request_head(stream, &WEB_HOST_HTTP_LIMITS)
        .await
        .context("read HTTP request")
}

pub struct HttpConnReader<R> {
    reader: BufReader<R>,
}

impl<R> HttpConnReader<R>
where
    R: AsyncRead + Unpin,
{
    pub fn new(stream: R) -> Self {
        Self {
            reader: BufReader::with_capacity(8 * 1024, stream),
        }
    }

    pub async fn next_request<W>(&mut self, writer: &mut W) -> Result<Option<HttpRequest>>
    where
        W: AsyncWrite + Unpin,
    {
        read_request(&mut self.reader, writer, &WEB_HOST_HTTP_LIMITS)
            .await
            .context("read HTTP request")
    }

    pub fn into_inner_and_leftover(self) -> (R, Vec<u8>) {
        let leftover = self.reader.buffer().to_vec();
        (self.reader.into_inner(), leftover)
    }
}

pub struct HttpResponse {
    pub status: u16,
    pub reason: &'static str,
    pub content_type: &'static str,
    pub extra_headers: Vec<(String, String)>,
    pub body: Bytes,
}

impl HttpResponse {
    pub fn ok(content_type: &'static str, body: impl Into<Bytes>) -> Self {
        Self {
            status: 200,
            reason: "OK",
            content_type,
            extra_headers: Vec::new(),
            body: body.into(),
        }
    }

    pub fn no_content() -> Self {
        Self {
            status: 204,
            reason: "No Content",
            content_type: "text/plain; charset=utf-8",
            extra_headers: Vec::new(),
            body: Bytes::new(),
        }
    }

    pub fn not_modified(etag: &str) -> Self {
        Self {
            status: 304,
            reason: "Not Modified",
            content_type: "text/plain; charset=utf-8",
            extra_headers: vec![
                ("ETag".to_string(), etag.to_string()),
                (
                    "Cache-Control".to_string(),
                    "public, max-age=0, must-revalidate".to_string(),
                ),
            ],
            body: Bytes::new(),
        }
    }

    pub fn text(status: u16, reason: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            reason,
            content_type: "text/plain; charset=utf-8",
            extra_headers: Vec::new(),
            body: Bytes::from(message.into()),
        }
    }

    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra_headers.push((name.into(), value.into()));
        self
    }

    pub async fn write_to<W>(&self, stream: &mut W, head_only: bool) -> Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        self.write_to_with_keep_alive(stream, head_only, false)
            .await
    }

    pub async fn write_to_with_keep_alive<W>(
        &self,
        stream: &mut W,
        head_only: bool,
        keep_alive: bool,
    ) -> Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        // RFC 9110: 204 No Content and 304 Not Modified MUST NOT include a body or Content-Length.
        let has_no_body = matches!(self.status, 204 | 304);
        let send_body = !head_only && !has_no_body && !self.body.is_empty();
        let coalesce_body = send_body && self.body.len() <= SMALL_BODY_COALESCE_BYTES;
        let mut wire = Vec::with_capacity(320 + if coalesce_body { self.body.len() } else { 0 });
        write_response_head_prefix(&mut wire, self.status, self.reason, keep_alive);
        use std::io::Write as _;
        if !has_no_body {
            let _ = write!(
                wire,
                "Content-Type: {}\r\nContent-Length: {}\r\n",
                self.content_type,
                self.body.len()
            );
        }
        for (name, value) in &self.extra_headers {
            let _ = write!(wire, "{name}: {value}\r\n");
        }
        wire.extend_from_slice(b"\r\n");

        if coalesce_body {
            wire.extend_from_slice(&self.body);
            stream
                .write_all(&wire)
                .await
                .context("write HTTP response")?;
        } else {
            stream
                .write_all(&wire)
                .await
                .context("write HTTP response headers")?;
            if send_body {
                stream
                    .write_all(&self.body)
                    .await
                    .context("write HTTP response body")?;
            }
        }
        stream.flush().await.context("flush HTTP response")?;
        Ok(())
    }
}

fn write_response_head_prefix(wire: &mut Vec<u8>, status: u16, reason: &str, keep_alive: bool) {
    let conn_headers = if keep_alive {
        "Connection: keep-alive\r\nKeep-Alive: timeout=30\r\n"
    } else {
        "Connection: close\r\n"
    };
    use std::io::Write as _;
    let _ = write!(
        wire,
        "HTTP/1.1 {status} {reason}\r\n\
         Cross-Origin-Opener-Policy: same-origin\r\n\
         Cross-Origin-Embedder-Policy: require-corp\r\n\
         Cross-Origin-Resource-Policy: same-origin\r\n\
         {conn_headers}"
    );
}

pub(crate) async fn write_streaming_ok_headers<W>(
    stream: &mut W,
    content_type: &'static str,
    content_length: Option<usize>,
    keep_alive: bool,
) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    let mut wire = Vec::with_capacity(320);
    write_response_head_prefix(&mut wire, 200, "OK", keep_alive);
    use std::io::Write as _;
    if let Some(len) = content_length {
        let _ = write!(
            wire,
            "Content-Type: {content_type}\r\nContent-Length: {len}\r\n\r\n"
        );
    } else {
        let _ = write!(
            wire,
            "Content-Type: {content_type}\r\nTransfer-Encoding: chunked\r\n\r\n"
        );
    }
    stream
        .write_all(&wire)
        .await
        .context("write streaming HTTP response headers")?;
    Ok(())
}
