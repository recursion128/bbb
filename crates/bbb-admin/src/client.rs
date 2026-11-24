//! The MCP Streamable HTTP client half.
//!
//! There is deliberately no second administration protocol: this client speaks
//! the same endpoint, the same session handshake and the same tools an agent
//! reaches, so a human and an agent cannot end up with different authority or
//! different semantics. What it adds is the client side of the transport — the
//! handshake order, the session header, and reading a server-sent event stream.

use std::{io, net::SocketAddr, sync::Arc, time::Duration};

#[cfg(test)]
use bbb_mcp::http::read_response_timeout;
use bbb_mcp::{
    http::{
        read_response_keep_alive_timeout, HttpError, HttpLimits, HttpResponse, SseOpenError,
        SseStream,
    },
    PROTOCOL_VERSION, PROTOCOL_VERSION_HEADER, SESSION_HEADER,
};
use rustls::{pki_types::ServerName, ClientConfig, RootCertStore};
use serde_json::{json, Map, Value};
use thiserror::Error;
use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt, BufReader},
    net::TcpStream,
    time::timeout,
};
use tokio_rustls::TlsConnector;
use url::Url;

/// The bound on one answer. A management answer is a bounded readout by
/// construction, so a larger body is a server this client should not be
/// talking to.
pub const MAX_ANSWER_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("{0} is not a management endpoint URL: {1}")]
    Endpoint(String, String),
    #[error("cannot reach {0}: {1}")]
    Connect(String, String),
    #[error("management transport failed: {0}")]
    Transport(String),
    #[error("the endpoint answered HTTP {status}: {detail}")]
    Http { status: u16, detail: String },
    #[error("{method} failed ({code}): {message}")]
    Rpc {
        method: String,
        code: i64,
        message: String,
        data: Option<Value>,
    },
    #[error("the endpoint speaks {found}, this client speaks {PROTOCOL_VERSION}")]
    ProtocolVersion { found: String },
    #[error("the endpoint assigned no session")]
    NoSession,
    #[error("TLS is not configured for {0}")]
    Tls(String),
}

#[derive(Debug, Clone)]
pub struct ClientConfiguration {
    /// The full endpoint URL, `http(s)://host[:port]/path`.
    pub endpoint: String,
    /// The bearer secret, read from a file rather than an argument.
    pub token: Option<String>,
    /// Extra certificate authorities, in DER, for a private TLS chain.
    pub certificate_authorities: Vec<Vec<u8>>,
    pub timeout: Duration,
    pub client_name: String,
    pub client_version: String,
}

impl Default for ClientConfiguration {
    fn default() -> Self {
        Self {
            endpoint: "http://127.0.0.1:25585/mcp".to_string(),
            token: None,
            certificate_authorities: Vec::new(),
            timeout: Duration::from_secs(30),
            client_name: "bbb-admin".to_string(),
            client_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

pub struct AdminClient {
    configuration: ClientConfiguration,
    host: String,
    port: u16,
    path: String,
    connector: Option<TlsConnector>,
    connection: Option<BufReader<Box<dyn Io>>>,
    session: Option<String>,
    next_id: i64,
}

fn answer_limits() -> HttpLimits {
    HttpLimits {
        max_body_bytes: MAX_ANSWER_BYTES,
        ..HttpLimits::default()
    }
}

trait Io: AsyncRead + AsyncWrite + Unpin + Send + Sync {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send + Sync> Io for T {}

impl AdminClient {
    pub fn new(configuration: ClientConfiguration) -> Result<Self, ClientError> {
        let url = Url::parse(&configuration.endpoint).map_err(|error| {
            ClientError::Endpoint(configuration.endpoint.clone(), error.to_string())
        })?;
        let tls = match url.scheme() {
            "http" => false,
            "https" => true,
            other => {
                return Err(ClientError::Endpoint(
                    configuration.endpoint.clone(),
                    format!("scheme {other} is neither http nor https"),
                ))
            }
        };
        let host = url
            .host_str()
            .ok_or_else(|| {
                ClientError::Endpoint(configuration.endpoint.clone(), "no host".to_string())
            })?
            .to_string();
        let port = url.port().unwrap_or(if tls { 443 } else { 80 });
        let path = match url.path() {
            "" | "/" => "/mcp".to_string(),
            path => path.to_string(),
        };
        let connector = tls
            .then(|| build_connector(&configuration.certificate_authorities))
            .transpose()?;
        Ok(Self {
            configuration,
            host,
            port,
            path,
            connector,
            connection: None,
            session: None,
            next_id: 1,
        })
    }

    pub fn session(&self) -> Option<&str> {
        self.session.as_deref()
    }

    /// The MCP handshake: initialize, check the revision, then say the session
    /// is ready. A client that skipped the notification would be one the server
    /// is entitled to treat as still initializing.
    pub async fn connect(&mut self) -> Result<Value, ClientError> {
        let handshake = self.rpc(
            "initialize",
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {
                    "name": self.configuration.client_name,
                    "version": self.configuration.client_version,
                },
            }),
        );
        let answer = self.send(&handshake, None).await?;
        // Whether the endpoint accepted the handshake at all is decided before
        // anything is read out of the answer: a refusal carries no session, and
        // reporting "no session" for a 401 would name the wrong problem.
        let result = decode_result("initialize", &answer)?;
        let session = answer
            .header(SESSION_HEADER)
            .ok_or(ClientError::NoSession)?
            .to_string();
        let found = result
            .get("protocolVersion")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if found != PROTOCOL_VERSION {
            return Err(ClientError::ProtocolVersion {
                found: found.to_string(),
            });
        }
        self.session = Some(session);
        self.notify("notifications/initialized", json!({})).await?;
        Ok(result)
    }

    pub async fn request(&mut self, method: &str, params: Value) -> Result<Value, ClientError> {
        let body = self.rpc(method, params);
        let answer = self.send(&body, self.session.clone()).await?;
        decode_result(method, &answer)
    }

    pub async fn notify(&mut self, method: &str, params: Value) -> Result<(), ClientError> {
        let body = json!({ "jsonrpc": "2.0", "method": method, "params": params });
        let answer = self.send(&body, self.session.clone()).await?;
        if answer.status != 202 && answer.status != 200 {
            return Err(ClientError::Http {
                status: answer.status,
                detail: answer.body_text(),
            });
        }
        Ok(())
    }

    /// Ends the session, which is what tells the server to forget it rather
    /// than waiting for it to expire.
    pub async fn disconnect(&mut self) -> Result<(), ClientError> {
        let Some(session) = self.session.take() else {
            return Ok(());
        };
        let mut request = format!("DELETE {} HTTP/1.1\r\n", self.path);
        request.push_str(&format!("host: {}:{}\r\n", self.host, self.port));
        request.push_str(&format!("{SESSION_HEADER}: {session}\r\n"));
        self.authorization(&mut request);
        request.push_str("connection: close\r\n\r\n");
        let _ = self.send_raw(request.as_bytes(), false).await?;
        Ok(())
    }

    pub async fn call_tool(
        &mut self,
        name: &str,
        arguments: Value,
        request_id: Option<&str>,
    ) -> Result<Value, ClientError> {
        let mut params = Map::new();
        params.insert("name".to_string(), Value::from(name));
        params.insert("arguments".to_string(), arguments);
        if let Some(request_id) = request_id {
            // The caller's own idempotency key: a retry that carries it is the
            // same operation rather than a second one.
            params.insert("_meta".to_string(), json!({ "bbb/requestId": request_id }));
        }
        self.request("tools/call", Value::Object(params)).await
    }

    pub async fn read_resource(&mut self, uri: &str) -> Result<Value, ClientError> {
        self.request("resources/read", json!({ "uri": uri })).await
    }

    /// Subscribes to every named resource and reads the event stream, handing
    /// each notification to `on_event` until it answers `false` or the stream
    /// ends.
    pub async fn watch(
        &mut self,
        uris: &[String],
        mut on_event: impl FnMut(Value) -> bool,
    ) -> Result<(), ClientError> {
        for uri in uris {
            self.request("resources/subscribe", json!({ "uri": uri }))
                .await?;
        }
        let Some(session) = self.session.clone() else {
            return Err(ClientError::NoSession);
        };
        let mut request = format!("GET {} HTTP/1.1\r\n", self.path);
        request.push_str(&format!("host: {}:{}\r\n", self.host, self.port));
        request.push_str("accept: text/event-stream\r\n");
        request.push_str(&format!("{SESSION_HEADER}: {session}\r\n"));
        request.push_str(&format!(
            "{PROTOCOL_VERSION_HEADER}: {PROTOCOL_VERSION}\r\n"
        ));
        self.authorization(&mut request);
        request.push_str("\r\n");

        let mut events = self.open_sse(request.as_bytes()).await?;

        while let Some(event) = events
            .next_event_timeout(self.configuration.timeout)
            .await
            .map_err(|error| ClientError::Transport(error.to_string()))?
        {
            let message = serde_json::from_str(&event.data).unwrap_or(Value::Null);
            if !on_event(message) {
                return Ok(());
            }
        }
        Ok(())
    }

    async fn open_sse(
        &mut self,
        request_bytes: &[u8],
    ) -> Result<SseStream<Box<dyn Io>>, ClientError> {
        if let Some(mut pooled) = self.connection.take() {
            if write_request(pooled.get_mut(), request_bytes, self.configuration.timeout)
                .await
                .is_ok()
            {
                match SseStream::open_buffered(pooled, &answer_limits(), self.configuration.timeout)
                    .await
                {
                    Ok(events) => return Ok(events),
                    Err(SseOpenError::Transport(err)) if is_stale_keep_alive_http(&err) => {}
                    Err(error) => return Err(map_sse_open_error(error)),
                }
            }
        }
        let mut reader = BufReader::new(self.open().await?);
        write_request(reader.get_mut(), request_bytes, self.configuration.timeout).await?;
        SseStream::open_buffered(reader, &answer_limits(), self.configuration.timeout)
            .await
            .map_err(map_sse_open_error)
    }

    fn rpc(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
    }

    fn authorization(&self, request: &mut String) {
        if let Some(token) = &self.configuration.token {
            request.push_str(&format!("authorization: Bearer {token}\r\n"));
        }
    }

    async fn send(
        &mut self,
        body: &Value,
        session: Option<String>,
    ) -> Result<HttpResponse, ClientError> {
        let body = body.to_string();
        let mut request = format!("POST {} HTTP/1.1\r\n", self.path);
        request.push_str(&format!("host: {}:{}\r\n", self.host, self.port));
        request.push_str("accept: application/json, text/event-stream\r\n");
        request.push_str("content-type: application/json\r\n");
        if let Some(session) = session {
            request.push_str(&format!("{SESSION_HEADER}: {session}\r\n"));
            request.push_str(&format!(
                "{PROTOCOL_VERSION_HEADER}: {PROTOCOL_VERSION}\r\n"
            ));
        }
        self.authorization(&mut request);
        request.push_str("connection: keep-alive\r\n");
        request.push_str(&format!("content-length: {}\r\n\r\n", body.len()));
        request.push_str(&body);

        self.send_raw(request.as_bytes(), true).await
    }

    async fn send_raw(
        &mut self,
        request_bytes: &[u8],
        keep_alive_requested: bool,
    ) -> Result<HttpResponse, ClientError> {
        if let Some(mut pooled) = self.connection.take() {
            if write_request(pooled.get_mut(), request_bytes, self.configuration.timeout)
                .await
                .is_ok()
            {
                match read_response_keep_alive_timeout(
                    &mut pooled,
                    &answer_limits(),
                    self.configuration.timeout,
                )
                .await
                {
                    Ok((response, server_keep_alive)) => {
                        if keep_alive_requested && server_keep_alive && pooled.buffer().is_empty() {
                            self.connection = Some(pooled);
                        }
                        return Ok(response);
                    }
                    Err(err) if is_stale_keep_alive_http(&err) => {}
                    Err(error) => return Err(ClientError::Transport(error.to_string())),
                }
            }
        }

        let mut reader = BufReader::new(self.open().await?);
        write_request(reader.get_mut(), request_bytes, self.configuration.timeout).await?;
        let (response, server_keep_alive) = read_response_keep_alive_timeout(
            &mut reader,
            &answer_limits(),
            self.configuration.timeout,
        )
        .await
        .map_err(|error| ClientError::Transport(error.to_string()))?;
        if keep_alive_requested && server_keep_alive && reader.buffer().is_empty() {
            self.connection = Some(reader);
        }
        Ok(response)
    }

    async fn open(&self) -> Result<Box<dyn Io>, ClientError> {
        let address = format!("{}:{}", self.host, self.port);
        let stream = timeout(self.configuration.timeout, TcpStream::connect(&address))
            .await
            .map_err(|_| ClientError::Connect(address.clone(), "timed out".to_string()))?
            .map_err(|error| ClientError::Connect(address.clone(), error.to_string()))?;
        match &self.connector {
            None => Ok(Box::new(stream)),
            Some(connector) => {
                let name = tls_server_name(&self.host)?;
                let stream = timeout(self.configuration.timeout, connector.connect(name, stream))
                    .await
                    .map_err(|_| {
                        ClientError::Connect(address.clone(), "TLS timed out".to_string())
                    })?
                    .map_err(|error| ClientError::Connect(address, error.to_string()))?;
                Ok(Box::new(stream))
            }
        }
    }
}

pub(crate) fn tls_server_name(host: &str) -> Result<ServerName<'static>, ClientError> {
    let bare = host
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .unwrap_or(host);
    ServerName::try_from(bare.to_string()).map_err(|_| ClientError::Tls(host.to_string()))
}

fn is_stale_keep_alive_http(error: &HttpError) -> bool {
    match error {
        HttpError::MalformedStatusLine => true,
        HttpError::Io(io_err) => matches!(
            io_err.kind(),
            io::ErrorKind::UnexpectedEof
                | io::ErrorKind::BrokenPipe
                | io::ErrorKind::ConnectionReset
                | io::ErrorKind::ConnectionAborted
        ),
        _ => false,
    }
}

fn map_sse_open_error(error: SseOpenError) -> ClientError {
    match error {
        SseOpenError::Http(response) => ClientError::Http {
            status: response.status,
            detail: response.body_text(),
        },
        SseOpenError::Transport(error) => ClientError::Transport(error.to_string()),
    }
}

/// Builds the TLS side of the client.
///
/// This client ships no bundled trust store. A management endpoint is named by
/// the operator, so its authority is named by the operator too, with
/// `--ca-file`; an empty root store would accept nothing, and pretending
/// otherwise would be the worse failure, so it is refused up front.
fn build_connector(authorities: &[Vec<u8>]) -> Result<TlsConnector, ClientError> {
    // The same explicit ring provider the rest of the workspace installs.
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut roots = RootCertStore::empty();
    let (loaded, ignored) = roots.add_parsable_certificates(
        authorities
            .iter()
            .map(|der| rustls::pki_types::CertificateDer::from(der.clone())),
    );
    if loaded == 0 {
        return Err(ClientError::Tls(format!(
            "https needs --ca-file: no usable certificate authority ({ignored} refused)"
        )));
    }
    let config = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok(TlsConnector::from(Arc::new(config)))
}

async fn write_request(
    stream: &mut (impl AsyncWrite + Unpin),
    bytes: &[u8],
    limit: Duration,
) -> Result<(), ClientError> {
    timeout(limit, async {
        stream.write_all(bytes).await?;
        stream.flush().await
    })
    .await
    .map_err(|_| {
        ClientError::Transport("the endpoint did not accept the request in time".to_string())
    })?
    .map_err(|error| ClientError::Transport(error.to_string()))
}

#[cfg(test)]
pub(crate) async fn read_answer(
    stream: &mut (impl AsyncRead + Unpin),
    limit: Duration,
) -> Result<HttpResponse, ClientError> {
    let mut reader = BufReader::new(stream);
    read_response_timeout(&mut reader, &answer_limits(), limit)
        .await
        .map_err(|error| ClientError::Transport(error.to_string()))
}

fn decode_result(method: &str, answer: &HttpResponse) -> Result<Value, ClientError> {
    let document: Value = serde_json::from_slice(&answer.body).unwrap_or(Value::Null);
    if let Some(error) = document.get("error") {
        return Err(ClientError::Rpc {
            method: method.to_string(),
            code: error
                .get("code")
                .and_then(Value::as_i64)
                .unwrap_or_default(),
            message: error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            data: error.get("data").cloned(),
        });
    }
    if answer.status != 200 {
        return Err(ClientError::Http {
            status: answer.status,
            detail: answer.body_text(),
        });
    }
    Ok(document.get("result").cloned().unwrap_or(Value::Null))
}

/// Where a management endpoint is, for a caller that has a socket address
/// rather than a URL.
pub fn endpoint_url(address: SocketAddr, path: &str, tls: bool) -> String {
    let scheme = if tls { "https" } else { "http" };
    format!("{scheme}://{address}{path}")
}
