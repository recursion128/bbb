//! The bounded HTTP/1.1 subset the MCP Streamable HTTP transport and its local
//! clients and gateways share.
//!
//! Vanilla 26.1 has no HTTP management surface at all: `ManagementServer`
//! upgrades one WebSocket and speaks JSON-RPC frames over it. MCP Streamable
//! HTTP is a request/response protocol with an optional server-sent event
//! stream, so bbb owns this layer outright.
//!
//! It is deliberately a *subset*. Only the codings an MCP peer actually needs
//! are accepted — an identity body with a `Content-Length`, a single-coding
//! `Transfer-Encoding: chunked` body (draining bounded trailers up to 4 KiB),
//! an SSE stream, or no body at all. Other transfer codings, mixed
//! `Transfer-Encoding` + `Content-Length`, and obs-fold continuations are
//! refused with an attributable error rather than parsed, and headers are
//! always split at the byte-level `\r\n\r\n` boundary before UTF-8 decoding so
//! binary bodies are never mangled and `Content-Length` responses never wait
//! for TCP EOF.

use std::{io, time::Duration};

use thiserror::Error;
use tokio::{
    io::{
        AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt,
        BufReader,
    },
    time::timeout,
};

/// What one HTTP message may cost before it is refused.
///
/// These are transport bounds, not policy: a request that fits still has to
/// pass authentication, scope and the world's own request validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HttpLimits {
    pub max_request_line_bytes: usize,
    pub max_header_bytes: usize,
    pub max_headers: usize,
    pub max_body_bytes: usize,
}

impl Default for HttpLimits {
    fn default() -> Self {
        Self {
            max_request_line_bytes: 8 * 1024,
            max_header_bytes: 16 * 1024,
            max_headers: 64,
            max_body_bytes: 1024 * 1024,
        }
    }
}

/// The methods the MCP endpoint distinguishes. Everything else is answered
/// `405` with an `Allow` header rather than routed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Head,
    Post,
    Delete,
    Options,
    Other,
}

impl HttpMethod {
    fn parse(token: &str) -> Self {
        match token {
            "GET" => Self::Get,
            "HEAD" => Self::Head,
            "POST" => Self::Post,
            "DELETE" => Self::Delete,
            "OPTIONS" => Self::Options,
            _ => Self::Other,
        }
    }
}

/// Case-insensitive HTTP headers in arrival order.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct HeaderMap {
    entries: Vec<(String, String)>,
}

impl HeaderMap {
    pub fn insert(&mut self, name: &str, value: impl Into<String>) {
        self.entries.push((name.to_ascii_lowercase(), value.into()));
    }

    /// The first value for a header name, which is what every header this
    /// transport reads is allowed to have.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.values(name).next()
    }

    pub fn values<'map>(&'map self, name: &str) -> impl Iterator<Item = &'map str> {
        let name = name.to_ascii_lowercase();
        self.entries
            .iter()
            .filter(move |(header, _)| *header == name)
            .map(|(_, value)| value.as_str())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
    }

    pub fn into_entries(self) -> Vec<(String, String)> {
        self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Whether a comma-separated list header carries a token, ignoring case and
    /// any `;q=` parameters.
    pub fn contains_token(&self, name: &str, token: &str) -> bool {
        self.values(name)
            .flat_map(|value| value.split(','))
            .map(list_item)
            .any(|item| item.eq_ignore_ascii_case(token))
    }

    /// Whether an `Accept` header admits a media type, honouring `*/*` and
    /// `type/*`. An absent `Accept` admits everything, as HTTP requires.
    pub fn accepts(&self, media_type: &str) -> bool {
        let mut declared = false;
        let (kind, subtype) = media_type.split_once('/').unwrap_or((media_type, "*"));
        for value in self.values("accept").flat_map(|value| value.split(',')) {
            declared = true;
            let item = list_item(value);
            if item == "*/*" || item.eq_ignore_ascii_case(media_type) {
                return true;
            }
            if let Some((accepted_kind, accepted_subtype)) = item.split_once('/') {
                if accepted_kind.eq_ignore_ascii_case(kind)
                    && (accepted_subtype == "*" || accepted_subtype.eq_ignore_ascii_case(subtype))
                {
                    return true;
                }
            }
        }
        !declared
    }
}

fn list_item(value: &str) -> &str {
    value
        .split(';')
        .next()
        .unwrap_or_default()
        .trim_matches(|character: char| character.is_ascii_whitespace())
}

/// One accepted request: head plus the buffered body (when read via [`read_request`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub method_name: String,
    pub target: String,
    pub path: String,
    pub query: Option<String>,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
    /// Whether the connection may carry another request after this one.
    pub keep_alive: bool,
}

impl HttpRequest {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name)
    }

    /// Decodes the first matching `application/x-www-form-urlencoded` query parameter.
    pub fn query_param(&self, key: &str) -> Option<String> {
        let query = self.query.as_deref()?;
        for pair in query.split('&') {
            if pair.is_empty() {
                continue;
            }
            let (raw_key, raw_val) = pair.split_once('=').unwrap_or((pair, ""));
            if decode_form_component(raw_key) == key {
                return Some(decode_form_component(raw_val));
            }
        }
        None
    }
}

fn decode_form_component(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                out.push(b' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                if let (Some(high), Some(low)) =
                    (hex_nibble(bytes[index + 1]), hex_nibble(bytes[index + 2]))
                {
                    out.push((high << 4) | low);
                    index += 3;
                } else {
                    out.push(bytes[index]);
                    index += 1;
                }
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[derive(Debug, Error)]
pub enum HttpError {
    #[error("management transport I/O failed: {0}")]
    Io(#[from] io::Error),
    #[error("management transport deadline elapsed")]
    TimedOut,
    #[error("the request line is malformed")]
    MalformedRequestLine,
    #[error("the request line exceeds {0} bytes")]
    RequestLineTooLarge(usize),
    #[error("the status line is malformed")]
    MalformedStatusLine,
    #[error("the status line exceeds {0} bytes")]
    StatusLineTooLarge(usize),
    #[error("a header line is malformed")]
    MalformedHeader,
    #[error("the HTTP headers exceed {0} bytes")]
    HeadersTooLarge(usize),
    #[error("the HTTP message carries more than {0} headers")]
    TooManyHeaders(usize),
    #[error("HTTP version {0} is not accepted")]
    UnsupportedVersion(String),
    #[error("only an identity or chunked transfer coding is accepted")]
    UnsupportedTransferCoding,
    #[error("the content length is malformed")]
    MalformedContentLength,
    #[error("the HTTP body exceeds {0} bytes")]
    BodyTooLarge(usize),
    #[error("the HTTP body is shorter than its content length")]
    TruncatedBody,
    #[error("unexpected response content-type {0}")]
    UnexpectedContentType(String),
}

impl HttpError {
    /// The status this failure is answered with when reading a client request,
    /// so a refusal is attributable rather than reported as a generic error.
    pub fn status(&self) -> u16 {
        match self {
            Self::Io(_)
            | Self::MalformedRequestLine
            | Self::MalformedHeader
            | Self::MalformedContentLength
            | Self::TruncatedBody => 400,
            Self::RequestLineTooLarge(_) => 414,
            Self::HeadersTooLarge(_) | Self::TooManyHeaders(_) => 431,
            Self::UnsupportedVersion(_) => 505,
            Self::UnsupportedTransferCoding => 411,
            Self::BodyTooLarge(_) => 413,
            Self::MalformedStatusLine
            | Self::StatusLineTooLarge(_)
            | Self::UnexpectedContentType(_) => 502,
            Self::TimedOut => 504,
        }
    }
}

/// Reads only the HTTP/1.1 request line and headers up to `\r\n\r\n`, returning
/// the parsed head (`body` empty) and any already-buffered bytes after the
/// header terminator (e.g. initial frames after a WebSocket upgrade).
/// Returns `None` when the peer closed the connection cleanly before sending a byte.
pub async fn read_request_head<R>(
    stream: &mut R,
    limits: &HttpLimits,
) -> Result<Option<(HttpRequest, Vec<u8>)>, HttpError>
where
    R: AsyncRead + Unpin,
{
    let mut reader = BufReader::with_capacity(2048, stream);
    let Some(request) = read_request_head_buffered(&mut reader, limits).await? else {
        return Ok(None);
    };
    let leftover = reader.buffer().to_vec();
    Ok(Some((request, leftover)))
}

async fn read_request_head_buffered<R>(
    reader: &mut R,
    limits: &HttpLimits,
) -> Result<Option<HttpRequest>, HttpError>
where
    R: AsyncBufRead + Unpin,
{
    let mut line = Vec::new();
    match read_line(reader, limits.max_request_line_bytes, &mut line).await? {
        LineOutcome::Complete => {}
        LineOutcome::TooLarge => {
            return Err(HttpError::RequestLineTooLarge(
                limits.max_request_line_bytes,
            ))
        }
        LineOutcome::Eof if line.is_empty() => return Ok(None),
        LineOutcome::Eof => return Err(HttpError::MalformedRequestLine),
    }
    let (method_name, target, default_keep_alive) = {
        let request_line =
            std::str::from_utf8(trim_line(&line)).map_err(|_| HttpError::MalformedRequestLine)?;
        let mut parts = request_line.split(' ');
        let (Some(method_name), Some(target), Some(version), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(HttpError::MalformedRequestLine);
        };
        if method_name.is_empty() || target.is_empty() {
            return Err(HttpError::MalformedRequestLine);
        }
        let default_keep_alive = parse_http_version(version)?;
        (
            method_name.to_string(),
            target.to_string(),
            default_keep_alive,
        )
    };

    let headers = read_headers(reader, limits, &mut line).await?;
    let keep_alive = resolve_keep_alive(&headers, default_keep_alive);
    let (path, query) = match target.split_once('?') {
        Some((path, query)) => (path.to_string(), Some(query.to_string())),
        None => (target.to_string(), None),
    };

    Ok(Some(HttpRequest {
        method: HttpMethod::parse(&method_name),
        method_name,
        target,
        path,
        query,
        headers,
        body: Vec::new(),
        keep_alive,
    }))
}

/// Parsed request head awaiting route/auth validation before sending
/// `100 Continue` or buffering any request body.
pub struct PendingHttpRequest {
    pub request: HttpRequest,
    pub content_length: usize,
    chunked: bool,
    max_body_bytes: usize,
    expects_continue: bool,
}

impl PendingHttpRequest {
    pub fn has_body(&self) -> bool {
        self.chunked || self.content_length > 0
    }
}

/// Reads and bounds only the HTTP/1.1 request head (`Transfer-Encoding` and
/// `Content-Length`), deferring `100 Continue` and body buffering until
/// authentication and origin checks succeed.
pub async fn read_request_head_checked<R>(
    reader: &mut R,
    limits: &HttpLimits,
) -> Result<Option<PendingHttpRequest>, HttpError>
where
    R: AsyncBufRead + Unpin,
{
    let Some(request) = read_request_head_buffered(reader, limits).await? else {
        return Ok(None);
    };
    let chunked = parse_transfer_encoding(&request.headers)?;
    if chunked && request.headers.values("content-length").next().is_some() {
        // RFC 9112 §6.3: reject messages carrying both Transfer-Encoding and Content-Length.
        return Err(HttpError::MalformedHeader);
    }
    let content_length = if chunked {
        0
    } else {
        parse_content_length(&request.headers, limits)?.unwrap_or(0)
    };
    let expects_continue =
        (chunked || content_length > 0) && request.headers.contains_token("expect", "100-continue");
    Ok(Some(PendingHttpRequest {
        request,
        content_length,
        chunked,
        max_body_bytes: limits.max_body_bytes,
        expects_continue,
    }))
}

/// Sends `100 Continue` when requested and buffers the validated request body.
pub async fn read_pending_body<R, W>(
    pending: PendingHttpRequest,
    reader: &mut R,
    writer: &mut W,
) -> Result<HttpRequest, HttpError>
where
    R: AsyncBufRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut request = pending.request;
    if pending.expects_continue {
        writer.write_all(b"HTTP/1.1 100 Continue\r\n\r\n").await?;
        writer.flush().await?;
    }
    if pending.chunked {
        request.body = read_chunked_body(reader, pending.max_body_bytes).await?;
    } else if pending.content_length > 0 {
        request.body = read_exact_body(reader, pending.content_length).await?;
    }
    Ok(request)
}

/// Reads one request, answering `Expect: 100-continue` before the body is
/// buffered. Returns `None` when the peer closed the connection cleanly between
/// requests.
pub async fn read_request<R, W>(
    reader: &mut R,
    writer: &mut W,
    limits: &HttpLimits,
) -> Result<Option<HttpRequest>, HttpError>
where
    R: AsyncBufRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let Some(pending) = read_request_head_checked(reader, limits).await? else {
        return Ok(None);
    };
    read_pending_body(pending, reader, writer).await.map(Some)
}

fn parse_http_version(version: &str) -> Result<bool, HttpError> {
    match version {
        "HTTP/1.1" => Ok(true),
        "HTTP/1.0" => Ok(false),
        other => Err(HttpError::UnsupportedVersion(other.to_string())),
    }
}

fn resolve_keep_alive(headers: &HeaderMap, default_keep_alive: bool) -> bool {
    if headers.contains_token("connection", "close") {
        false
    } else if headers.contains_token("connection", "keep-alive") {
        true
    } else {
        default_keep_alive
    }
}

async fn read_headers<R: AsyncBufRead + Unpin>(
    reader: &mut R,
    limits: &HttpLimits,
    line: &mut Vec<u8>,
) -> Result<HeaderMap, HttpError> {
    let mut headers = HeaderMap::default();
    let mut header_budget = limits.max_header_bytes;
    loop {
        match read_line(reader, header_budget, line).await? {
            LineOutcome::Complete => {}
            LineOutcome::TooLarge => {
                return Err(HttpError::HeadersTooLarge(limits.max_header_bytes))
            }
            LineOutcome::Eof => return Err(HttpError::MalformedHeader),
        }
        header_budget = header_budget
            .checked_sub(line.len())
            .ok_or(HttpError::HeadersTooLarge(limits.max_header_bytes))?;
        let header_line = trim_line(line);
        if header_line.is_empty() {
            break;
        }
        if headers.len() == limits.max_headers {
            return Err(HttpError::TooManyHeaders(limits.max_headers));
        }
        let header_line =
            std::str::from_utf8(header_line).map_err(|_| HttpError::MalformedHeader)?;
        // An obs-fold continuation starts with whitespace and has no name; it is
        // refused rather than joined, because joining is what makes request
        // smuggling possible.
        if header_line.starts_with(' ') || header_line.starts_with('\t') {
            return Err(HttpError::MalformedHeader);
        }
        let (name, value) = header_line
            .split_once(':')
            .ok_or(HttpError::MalformedHeader)?;
        if name.is_empty() || name.contains(char::is_whitespace) {
            return Err(HttpError::MalformedHeader);
        }
        headers.insert(
            name,
            value.trim_matches(|character: char| character == ' ' || character == '\t'),
        );
    }
    Ok(headers)
}

fn parse_content_length(
    headers: &HeaderMap,
    limits: &HttpLimits,
) -> Result<Option<usize>, HttpError> {
    let mut content_lengths = headers.values("content-length");
    match content_lengths.next() {
        Some(value) => {
            // Two content lengths are a smuggling attempt even when they agree.
            if content_lengths.next().is_some() {
                return Err(HttpError::MalformedContentLength);
            }
            if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(HttpError::MalformedContentLength);
            }
            let length = value
                .parse::<usize>()
                .map_err(|_| HttpError::MalformedContentLength)?;
            if length > limits.max_body_bytes {
                return Err(HttpError::BodyTooLarge(limits.max_body_bytes));
            }
            Ok(Some(length))
        }
        None => Ok(None),
    }
}

fn parse_transfer_encoding(headers: &HeaderMap) -> Result<bool, HttpError> {
    let mut values = headers.values("transfer-encoding").peekable();
    if values.peek().is_none() {
        return Ok(false);
    }
    let mut codings = Vec::new();
    for raw in values {
        for token in raw.split(',') {
            let trimmed = token.trim();
            if !trimmed.is_empty() {
                codings.push(trimmed);
            }
        }
    }
    if codings.len() == 1 && codings[0].eq_ignore_ascii_case("chunked") {
        Ok(true)
    } else {
        Err(HttpError::UnsupportedTransferCoding)
    }
}

const MAX_CHUNK_LINE_BYTES: usize = 128;
const MAX_TRAILER_BYTES: usize = 4096;

async fn read_chunked_body<R: AsyncBufRead + Unpin>(
    reader: &mut R,
    max_body_bytes: usize,
) -> Result<Vec<u8>, HttpError> {
    let mut body = Vec::new();
    let mut line = Vec::new();
    loop {
        match read_line(reader, MAX_CHUNK_LINE_BYTES, &mut line).await? {
            LineOutcome::Complete => {}
            LineOutcome::TooLarge => return Err(HttpError::MalformedContentLength),
            LineOutcome::Eof => return Err(HttpError::TruncatedBody),
        }
        if !line.ends_with(b"\r\n") {
            return Err(HttpError::MalformedContentLength);
        }
        let size_line =
            std::str::from_utf8(trim_line(&line)).map_err(|_| HttpError::MalformedContentLength)?;
        let size_token = size_line.split(';').next().unwrap_or("").trim();
        if size_token.is_empty() || !size_token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(HttpError::MalformedContentLength);
        }
        let chunk_size =
            usize::from_str_radix(size_token, 16).map_err(|_| HttpError::MalformedContentLength)?;
        if chunk_size == 0 {
            // Drain optional trailers up to the terminating empty CRLF line.
            let mut trailer_budget = MAX_TRAILER_BYTES;
            loop {
                match read_line(reader, trailer_budget, &mut line).await? {
                    LineOutcome::Complete => {}
                    LineOutcome::TooLarge => {
                        return Err(HttpError::HeadersTooLarge(MAX_TRAILER_BYTES))
                    }
                    LineOutcome::Eof => return Err(HttpError::TruncatedBody),
                }
                if !line.ends_with(b"\r\n") {
                    return Err(HttpError::MalformedHeader);
                }
                trailer_budget = trailer_budget
                    .checked_sub(line.len())
                    .ok_or(HttpError::HeadersTooLarge(MAX_TRAILER_BYTES))?;
                if trim_line(&line).is_empty() {
                    break;
                }
            }
            return Ok(body);
        }
        let next_len = body
            .len()
            .checked_add(chunk_size)
            .ok_or(HttpError::BodyTooLarge(max_body_bytes))?;
        if next_len > max_body_bytes {
            return Err(HttpError::BodyTooLarge(max_body_bytes));
        }
        let start = body.len();
        body.resize(next_len, 0);
        reader
            .read_exact(&mut body[start..])
            .await
            .map_err(|error| {
                if error.kind() == io::ErrorKind::UnexpectedEof {
                    HttpError::TruncatedBody
                } else {
                    HttpError::Io(error)
                }
            })?;
        let mut crlf = [0u8; 2];
        reader.read_exact(&mut crlf).await.map_err(|error| {
            if error.kind() == io::ErrorKind::UnexpectedEof {
                HttpError::TruncatedBody
            } else {
                HttpError::Io(error)
            }
        })?;
        if &crlf != b"\r\n" {
            return Err(HttpError::MalformedContentLength);
        }
    }
}

async fn read_exact_body<R: AsyncBufRead + Unpin>(
    reader: &mut R,
    content_length: usize,
) -> Result<Vec<u8>, HttpError> {
    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body).await.map_err(|error| {
            if error.kind() == io::ErrorKind::UnexpectedEof {
                HttpError::TruncatedBody
            } else {
                HttpError::Io(error)
            }
        })?;
    }
    Ok(body)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineOutcome {
    Complete,
    /// End of input, with whatever partial bytes were read left in `line`.
    Eof,
    TooLarge,
}

/// Reads one line including its terminator, never buffering past `limit`.
pub async fn read_line<R: AsyncBufRead + Unpin>(
    reader: &mut R,
    limit: usize,
    line: &mut Vec<u8>,
) -> Result<LineOutcome, io::Error> {
    line.clear();
    loop {
        let (complete, consumed, outcome) = {
            let available = reader.fill_buf().await?;
            if available.is_empty() {
                return Ok(LineOutcome::Eof);
            }
            match available.iter().position(|byte| *byte == b'\n') {
                Some(index) => {
                    let len = index + 1;
                    if line.len() + len > limit {
                        let take = limit.saturating_sub(line.len());
                        line.extend_from_slice(&available[..take]);
                        (false, take, Some(LineOutcome::TooLarge))
                    } else {
                        line.extend_from_slice(&available[..len]);
                        (true, len, None)
                    }
                }
                None => {
                    if line.len() + available.len() > limit {
                        let take = limit.saturating_sub(line.len());
                        line.extend_from_slice(&available[..take]);
                        (false, take, Some(LineOutcome::TooLarge))
                    } else {
                        line.extend_from_slice(available);
                        (false, available.len(), None)
                    }
                }
            }
        };
        reader.consume(consumed);
        if let Some(outcome) = outcome {
            return Ok(outcome);
        }
        if complete {
            return Ok(LineOutcome::Complete);
        }
    }
}

pub fn trim_line(line: &[u8]) -> &[u8] {
    let line = line.strip_suffix(b"\n").unwrap_or(line);
    line.strip_suffix(b"\r").unwrap_or(line)
}

/// Parsed HTTP/1.x status line and headers before reading any body or SSE stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponseHead {
    pub version: String,
    pub status: u16,
    pub reason: String,
    pub headers: HeaderMap,
    pub keep_alive: bool,
}

impl HttpResponseHead {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name)
    }
}

/// Reads an HTTP/1.x response status line and header block up to `\r\n\r\n`,
/// splitting at byte level before UTF-8 decoding headers and leaving `reader`
/// positioned at the exact start of the body or event stream.
pub async fn read_response_head<R>(
    reader: &mut R,
    limits: &HttpLimits,
) -> Result<HttpResponseHead, HttpError>
where
    R: AsyncBufRead + Unpin,
{
    let mut line = Vec::new();
    match read_line(reader, limits.max_request_line_bytes, &mut line).await? {
        LineOutcome::Complete => {}
        LineOutcome::TooLarge => {
            return Err(HttpError::StatusLineTooLarge(limits.max_request_line_bytes))
        }
        LineOutcome::Eof => return Err(HttpError::MalformedStatusLine),
    }

    let (version, status, reason, default_keep_alive) = {
        let status_line =
            std::str::from_utf8(trim_line(&line)).map_err(|_| HttpError::MalformedStatusLine)?;
        let Some((version, rest)) = status_line.split_once(' ') else {
            return Err(HttpError::MalformedStatusLine);
        };
        let default_keep_alive = parse_http_version(version)?;
        let (status_token, reason) = match rest.split_once(' ') {
            Some((code, reason)) => (code, reason),
            None => (rest, ""),
        };
        if status_token.len() != 3 || !status_token.bytes().all(|b| b.is_ascii_digit()) {
            return Err(HttpError::MalformedStatusLine);
        }
        let status = status_token
            .parse::<u16>()
            .map_err(|_| HttpError::MalformedStatusLine)?;
        if !(100..=599).contains(&status) {
            return Err(HttpError::MalformedStatusLine);
        }
        (
            version.to_string(),
            status,
            reason.to_string(),
            default_keep_alive,
        )
    };

    let headers = read_headers(reader, limits, &mut line).await?;
    let keep_alive = resolve_keep_alive(&headers, default_keep_alive);
    Ok(HttpResponseHead {
        version,
        status,
        reason,
        headers,
        keep_alive,
    })
}

/// Reads an HTTP/1.x response head bounded by `deadline`.
pub async fn read_response_head_timeout<R>(
    reader: &mut R,
    limits: &HttpLimits,
    deadline: Duration,
) -> Result<HttpResponseHead, HttpError>
where
    R: AsyncBufRead + Unpin,
{
    timeout(deadline, read_response_head(reader, limits))
        .await
        .map_err(|_| HttpError::TimedOut)?
}

async fn read_response_body<R>(
    reader: &mut R,
    head: &HttpResponseHead,
    limits: &HttpLimits,
) -> Result<Vec<u8>, HttpError>
where
    R: AsyncBufRead + Unpin,
{
    if (100..200).contains(&head.status) || head.status == 204 || head.status == 304 {
        return Ok(Vec::new());
    }
    if parse_transfer_encoding(&head.headers)? {
        if head.headers.values("content-length").next().is_some() {
            return Err(HttpError::MalformedHeader);
        }
        return read_chunked_body(reader, limits.max_body_bytes).await;
    }
    if let Some(content_length) = parse_content_length(&head.headers, limits)? {
        return read_exact_body(reader, content_length).await;
    }

    let mut body = Vec::new();
    let mut chunk = [0u8; 8 * 1024];
    loop {
        let read = reader.read(&mut chunk).await?;
        if read == 0 {
            return Ok(body);
        }
        if body.len().saturating_add(read) > limits.max_body_bytes {
            return Err(HttpError::BodyTooLarge(limits.max_body_bytes));
        }
        body.extend_from_slice(&chunk[..read]);
    }
}

/// Reads one complete HTTP/1.x response (skipping any `100 Continue` interim
/// head), framing by `Content-Length` without waiting for TCP EOF so keep-alive
/// peers return immediately and binary bodies stay untouched.
pub async fn read_response<R>(
    reader: &mut R,
    limits: &HttpLimits,
) -> Result<HttpResponse, HttpError>
where
    R: AsyncBufRead + Unpin,
{
    read_response_inner(reader, limits)
        .await
        .map(|(resp, _)| resp)
}

async fn read_response_inner<R>(
    reader: &mut R,
    limits: &HttpLimits,
) -> Result<(HttpResponse, bool), HttpError>
where
    R: AsyncBufRead + Unpin,
{
    let head = loop {
        let head = read_response_head(reader, limits).await?;
        if head.status != 100 {
            break head;
        }
    };
    let keep_alive = head.keep_alive;
    let body = read_response_body(reader, &head, limits).await?;
    Ok((
        HttpResponse {
            status: head.status,
            headers: head.headers.into_entries(),
            body,
        },
        keep_alive,
    ))
}

/// Reads one complete HTTP/1.x response from a buffered reader within `deadline`.
pub async fn read_response_timeout<R>(
    reader: &mut R,
    limits: &HttpLimits,
    deadline: Duration,
) -> Result<HttpResponse, HttpError>
where
    R: AsyncBufRead + Unpin,
{
    timeout(deadline, read_response(reader, limits))
        .await
        .map_err(|_| HttpError::TimedOut)?
}

/// Reads one complete HTTP/1.x response from a buffered reader within `deadline`,
/// also returning whether the peer left the connection keep-alive.
pub async fn read_response_keep_alive_timeout<R>(
    reader: &mut R,
    limits: &HttpLimits,
    deadline: Duration,
) -> Result<(HttpResponse, bool), HttpError>
where
    R: AsyncBufRead + Unpin,
{
    timeout(deadline, read_response_inner(reader, limits))
        .await
        .map_err(|_| HttpError::TimedOut)?
}

/// Reads one complete HTTP/1.x response from an unbuffered stream within `deadline`.
pub async fn read_one_response<S>(
    stream: &mut S,
    limits: &HttpLimits,
    deadline: Duration,
) -> Result<HttpResponse, HttpError>
where
    S: AsyncRead + Unpin,
{
    let mut reader = BufReader::new(stream);
    read_response_timeout(&mut reader, limits, deadline).await
}

/// One answer. The body is buffered and written with an explicit content length
/// (except on `304 Not Modified`), so a client never has to guess where an
/// answer ends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn new(status: u16) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    pub fn json(status: u16, body: Vec<u8>) -> Self {
        Self::new(status)
            .with_header("content-type", "application/json")
            .with_body(body)
    }

    pub fn text(status: u16, body: &str) -> Self {
        Self::new(status)
            .with_header("content-type", "text/plain; charset=utf-8")
            .with_body(body.as_bytes().to_vec())
    }

    pub fn with_header(mut self, name: &str, value: impl Into<String>) -> Self {
        self.headers.push((name.to_string(), value.into()));
        self
    }

    pub fn with_body(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header, _)| header.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    pub fn body_text(&self) -> String {
        String::from_utf8_lossy(&self.body).trim().to_string()
    }
}

pub fn reason_phrase(status: u16) -> &'static str {
    match status {
        100 => "Continue",
        101 => "Switching Protocols",
        200 => "OK",
        202 => "Accepted",
        204 => "No Content",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        406 => "Not Acceptable",
        409 => "Conflict",
        411 => "Length Required",
        413 => "Content Too Large",
        414 => "URI Too Long",
        415 => "Unsupported Media Type",
        429 => "Too Many Requests",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        505 => "HTTP Version Not Supported",
        _ => "Unknown",
    }
}

fn validate_response_header(name: &str, value: &str) -> io::Result<()> {
    if name.is_empty()
        || name
            .bytes()
            .any(|byte| !byte.is_ascii_graphic() || byte == b':')
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid HTTP response header name",
        ));
    }
    if value
        .bytes()
        .any(|byte| byte == b'\r' || byte == b'\n' || byte == b'\0')
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid HTTP response header value",
        ));
    }
    Ok(())
}

pub async fn write_response<W: AsyncWrite + Unpin>(
    writer: &mut W,
    response: &HttpResponse,
    keep_alive: bool,
    include_body: bool,
) -> io::Result<()> {
    let mut head = format!(
        "HTTP/1.1 {} {}\r\n",
        response.status,
        reason_phrase(response.status)
    );
    let mut has_content_length = false;
    for (name, value) in &response.headers {
        validate_response_header(name, value)?;
        if name.eq_ignore_ascii_case("content-length") {
            has_content_length = true;
        }
        head.push_str(name);
        head.push_str(": ");
        head.push_str(value);
        head.push_str("\r\n");
    }
    let bodyless =
        (100..200).contains(&response.status) || response.status == 204 || response.status == 304;
    if !bodyless && !has_content_length {
        head.push_str(&format!("content-length: {}\r\n", response.body.len()));
    }
    head.push_str(if keep_alive {
        "connection: keep-alive\r\n"
    } else {
        "connection: close\r\n"
    });
    head.push_str("\r\n");
    writer.write_all(head.as_bytes()).await?;
    if include_body && !bodyless && !response.body.is_empty() {
        writer.write_all(&response.body).await?;
    }
    writer.flush().await
}

/// A server-sent event stream, which is how an MCP server delivers messages a
/// client did not ask for — resource update notifications, in this server.
pub struct EventStream<W> {
    writer: W,
}

impl<W: AsyncWrite + Unpin> EventStream<W> {
    /// Writes the response head and leaves the connection open for events.
    pub async fn start(mut writer: W, headers: &[(String, String)]) -> io::Result<Self> {
        let mut head = String::from("HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n");
        head.push_str("cache-control: no-store\r\n");
        head.push_str("connection: close\r\n");
        for (name, value) in headers {
            validate_response_header(name, value)?;
            head.push_str(name);
            head.push_str(": ");
            head.push_str(value);
            head.push_str("\r\n");
        }
        head.push_str("\r\n");
        writer.write_all(head.as_bytes()).await?;
        writer.flush().await?;
        Ok(Self { writer })
    }

    pub async fn send(&mut self, id: Option<u64>, data: &str) -> io::Result<()> {
        let mut frame = String::new();
        if let Some(id) = id {
            frame.push_str(&format!("id: {id}\n"));
        }
        frame.push_str("event: message\n");
        // WHATWG SSE treats LF, CRLF, and bare CR as line terminators, so split
        // all three to prevent an embedded `\r` from injecting SSE fields.
        if data.contains('\r') {
            let normalized = data.replace("\r\n", "\n");
            for line in normalized.split(['\n', '\r']) {
                frame.push_str("data: ");
                frame.push_str(line);
                frame.push('\n');
            }
        } else {
            for line in data.split('\n') {
                frame.push_str("data: ");
                frame.push_str(line);
                frame.push('\n');
            }
        }
        frame.push('\n');
        self.writer.write_all(frame.as_bytes()).await?;
        self.writer.flush().await
    }

    /// A comment frame, which keeps an idle stream and its intermediaries alive
    /// without delivering a message.
    pub async fn keep_alive(&mut self) -> io::Result<()> {
        self.writer.write_all(b": keep-alive\n\n").await?;
        self.writer.flush().await
    }

    /// Closes the stream at the transport's own layer. Over TLS this is what
    /// sends `close_notify`, which is the difference between a stream that
    /// ended and one that was cut.
    pub async fn finish(mut self) -> io::Result<()> {
        self.writer.shutdown().await
    }
}

/// One decoded Server-Sent Event frame delivered to an SSE client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseEvent {
    pub id: Option<String>,
    pub event: Option<String>,
    pub data: String,
}

/// Failure while opening a client-side `text/event-stream` response.
#[derive(Debug, Error)]
pub enum SseOpenError {
    #[error(transparent)]
    Transport(#[from] HttpError),
    #[error("the endpoint answered HTTP {} instead of an event stream", .0.status)]
    Http(HttpResponse),
}

/// Bounded client-side reader for an HTTP `text/event-stream` response.
pub struct SseStream<R> {
    reader: BufReader<R>,
    head: HttpResponseHead,
    max_event_bytes: usize,
    chunked: bool,
    chunk_remaining: usize,
    chunk_need_crlf: bool,
    chunk_eof: bool,
    chunk_buf: Vec<u8>,
    chunk_buf_pos: usize,
}

impl<R: AsyncRead + Unpin> SseStream<R> {
    /// Reads the HTTP status line and response headers within `head_timeout`
    /// before streaming events. Non-`200` responses read their bounded body and
    /// return [`SseOpenError::Http`].
    pub async fn open(
        stream: R,
        limits: &HttpLimits,
        head_timeout: Duration,
    ) -> Result<Self, SseOpenError> {
        Self::open_buffered(BufReader::new(stream), limits, head_timeout).await
    }

    /// Variant of [`Self::open`] reusing an existing [`BufReader`] on a keep-alive stream.
    pub async fn open_buffered(
        mut reader: BufReader<R>,
        limits: &HttpLimits,
        head_timeout: Duration,
    ) -> Result<Self, SseOpenError> {
        let (head, chunked) = timeout(head_timeout, async {
            loop {
                let head = read_response_head(&mut reader, limits).await?;
                if head.status == 100 {
                    continue;
                }
                if head.status != 200 {
                    let body = read_response_body(&mut reader, &head, limits).await?;
                    return Err(SseOpenError::Http(HttpResponse {
                        status: head.status,
                        headers: head.headers.into_entries(),
                        body,
                    }));
                }
                let content_type = head.header("content-type").unwrap_or_default();
                let media_type = list_item(content_type);
                if !media_type.eq_ignore_ascii_case("text/event-stream") {
                    return Err(SseOpenError::Transport(HttpError::UnexpectedContentType(
                        content_type.to_string(),
                    )));
                }
                let chunked = parse_transfer_encoding(&head.headers)?;
                if chunked && head.headers.values("content-length").next().is_some() {
                    return Err(SseOpenError::Transport(HttpError::MalformedHeader));
                }
                return Ok((head, chunked));
            }
        })
        .await
        .map_err(|_| SseOpenError::Transport(HttpError::TimedOut))??;

        Ok(Self {
            reader,
            head,
            max_event_bytes: limits.max_body_bytes,
            chunked,
            chunk_remaining: 0,
            chunk_need_crlf: false,
            chunk_eof: false,
            chunk_buf: Vec::new(),
            chunk_buf_pos: 0,
        })
    }

    pub fn head(&self) -> &HttpResponseHead {
        &self.head
    }

    /// Reads the next event frame, ignoring comment keep-alives (`: ...`).
    /// Returns `Ok(None)` when the stream closes cleanly between events.
    pub async fn next_event(&mut self) -> Result<Option<SseEvent>, HttpError> {
        self.next_event_inner(None).await
    }

    /// Reads the next event frame, timing out if no line (including `: keep-alive`
    /// comments) arrives within `idle_timeout`.
    pub async fn next_event_timeout(
        &mut self,
        idle_timeout: Duration,
    ) -> Result<Option<SseEvent>, HttpError> {
        self.next_event_inner(Some(idle_timeout)).await
    }

    async fn read_chunked_sse_line(
        &mut self,
        limit: usize,
        out: &mut Vec<u8>,
    ) -> Result<LineOutcome, HttpError> {
        out.clear();
        let mut header_line = Vec::new();
        loop {
            if self.chunk_buf_pos < self.chunk_buf.len() {
                let available = &self.chunk_buf[self.chunk_buf_pos..];
                match available.iter().position(|byte| *byte == b'\n') {
                    Some(index) => {
                        let len = index + 1;
                        if out.len() + len > limit {
                            let take = limit.saturating_sub(out.len());
                            out.extend_from_slice(&available[..take]);
                            self.chunk_buf_pos += take;
                            return Ok(LineOutcome::TooLarge);
                        }
                        out.extend_from_slice(&available[..len]);
                        self.chunk_buf_pos += len;
                        if self.chunk_buf_pos == self.chunk_buf.len() {
                            self.chunk_buf.clear();
                            self.chunk_buf_pos = 0;
                        }
                        return Ok(LineOutcome::Complete);
                    }
                    None => {
                        if out.len() + available.len() > limit {
                            let take = limit.saturating_sub(out.len());
                            out.extend_from_slice(&available[..take]);
                            self.chunk_buf_pos += take;
                            return Ok(LineOutcome::TooLarge);
                        }
                        out.extend_from_slice(available);
                        self.chunk_buf.clear();
                        self.chunk_buf_pos = 0;
                    }
                }
            }
            if self.chunk_eof {
                return Ok(LineOutcome::Eof);
            }
            if self.chunk_remaining == 0 {
                if self.chunk_need_crlf {
                    let mut crlf = [0u8; 2];
                    self.reader.read_exact(&mut crlf).await.map_err(|error| {
                        if error.kind() == io::ErrorKind::UnexpectedEof {
                            HttpError::TruncatedBody
                        } else {
                            HttpError::Io(error)
                        }
                    })?;
                    if &crlf != b"\r\n" {
                        return Err(HttpError::MalformedContentLength);
                    }
                    self.chunk_need_crlf = false;
                }
                match read_line(&mut self.reader, MAX_CHUNK_LINE_BYTES, &mut header_line).await? {
                    LineOutcome::Complete => {}
                    LineOutcome::TooLarge => return Err(HttpError::MalformedContentLength),
                    LineOutcome::Eof => return Err(HttpError::TruncatedBody),
                }
                if !header_line.ends_with(b"\r\n") {
                    return Err(HttpError::MalformedContentLength);
                }
                let size_line = std::str::from_utf8(trim_line(&header_line))
                    .map_err(|_| HttpError::MalformedContentLength)?;
                let size_token = size_line.split(';').next().unwrap_or("").trim();
                if size_token.is_empty() || !size_token.bytes().all(|byte| byte.is_ascii_hexdigit())
                {
                    return Err(HttpError::MalformedContentLength);
                }
                let chunk_size = usize::from_str_radix(size_token, 16)
                    .map_err(|_| HttpError::MalformedContentLength)?;
                if chunk_size == 0 {
                    let mut trailer_budget = MAX_TRAILER_BYTES;
                    loop {
                        match read_line(&mut self.reader, trailer_budget, &mut header_line).await? {
                            LineOutcome::Complete => {}
                            LineOutcome::TooLarge => {
                                return Err(HttpError::HeadersTooLarge(MAX_TRAILER_BYTES))
                            }
                            LineOutcome::Eof => return Err(HttpError::TruncatedBody),
                        }
                        if !header_line.ends_with(b"\r\n") {
                            return Err(HttpError::MalformedHeader);
                        }
                        trailer_budget = trailer_budget
                            .checked_sub(header_line.len())
                            .ok_or(HttpError::HeadersTooLarge(MAX_TRAILER_BYTES))?;
                        if trim_line(&header_line).is_empty() {
                            break;
                        }
                    }
                    self.chunk_eof = true;
                    return Ok(LineOutcome::Eof);
                }
                self.chunk_remaining = chunk_size;
                self.chunk_need_crlf = true;
            }
            let mut tmp = [0u8; 4096];
            let to_read = self.chunk_remaining.min(tmp.len());
            let read = self.reader.read(&mut tmp[..to_read]).await?;
            if read == 0 {
                return Err(HttpError::TruncatedBody);
            }
            self.chunk_remaining -= read;
            self.chunk_buf.extend_from_slice(&tmp[..read]);
        }
    }

    async fn read_sse_line(
        &mut self,
        limit: usize,
        out: &mut Vec<u8>,
    ) -> Result<LineOutcome, HttpError> {
        if self.chunked {
            self.read_chunked_sse_line(limit, out).await
        } else {
            Ok(read_line(&mut self.reader, limit, out).await?)
        }
    }

    async fn next_event_inner(
        &mut self,
        idle_timeout: Option<Duration>,
    ) -> Result<Option<SseEvent>, HttpError> {
        let mut line = Vec::new();
        let mut id: Option<String> = None;
        let mut event: Option<String> = None;
        let mut data = String::new();
        let mut has_data = false;
        let mut accumulated_bytes = 0usize;

        loop {
            let line_budget = self
                .max_event_bytes
                .checked_sub(accumulated_bytes)
                .ok_or(HttpError::BodyTooLarge(self.max_event_bytes))?;
            let line_outcome = match idle_timeout {
                Some(limit) => timeout(limit, self.read_sse_line(line_budget, &mut line))
                    .await
                    .map_err(|_| HttpError::TimedOut)??,
                None => self.read_sse_line(line_budget, &mut line).await?,
            };
            match line_outcome {
                LineOutcome::Complete => {}
                LineOutcome::TooLarge => return Err(HttpError::BodyTooLarge(self.max_event_bytes)),
                LineOutcome::Eof => {
                    if line.is_empty() && !has_data {
                        return Ok(None);
                    }
                    return Err(HttpError::TruncatedBody);
                }
            }
            accumulated_bytes = accumulated_bytes.saturating_add(line.len());
            let trimmed =
                std::str::from_utf8(trim_line(&line)).map_err(|_| HttpError::MalformedHeader)?;
            if trimmed.is_empty() {
                if has_data {
                    return Ok(Some(SseEvent { id, event, data }));
                }
                id = None;
                event = None;
                accumulated_bytes = 0;
                continue;
            }
            if trimmed.starts_with(':') {
                if !has_data {
                    accumulated_bytes = 0;
                }
                continue;
            }
            let (field, raw_value) = match trimmed.split_once(':') {
                Some((field, rest)) => (field, rest.strip_prefix(' ').unwrap_or(rest)),
                None => (trimmed, ""),
            };
            match field {
                "data" => {
                    if has_data {
                        data.push('\n');
                    }
                    data.push_str(raw_value);
                    has_data = true;
                }
                "event" => event = Some(raw_value.to_string()),
                "id" if !raw_value.contains('\0') => id = Some(raw_value.to_string()),
                _ => {}
            }
        }
    }
}
