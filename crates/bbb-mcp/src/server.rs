//! The MCP Streamable HTTP endpoint.
//!
//! Vanilla owner: `ManagementServer`, which binds one socket, authenticates the
//! upgrade and hands frames to `IncomingRpcMethods`. This server binds one
//! socket too, but speaks the protocol revision named in [`PROTOCOL_VERSION`]:
//! one endpoint that accepts `POST` for requests, `GET` for the server-to-client
//! event stream, and `DELETE` to end a session.
//!
//! The version is pinned, not negotiated open-endedly. A client that asks for a
//! revision this server does not implement is answered with the one it does,
//! which is what lets a conformance fixture assert an exact wire shape.

use std::{
    collections::{HashMap, VecDeque},
    future::Future,
    io,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex, PoisonError},
    time::{Duration, Instant},
};

use serde_json::{json, Map, Value};
use thiserror::Error;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::{OwnedSemaphorePermit, Semaphore},
    time::timeout,
};

use crate::{
    auth::{
        authenticate_async_with_oauth, challenge, origin_allowed, AuthConfig, AuthFailure,
        Principal,
    },
    host::{CallRefusal, Caller, McpHost, PendingCall, ScopeLadder, ScopeOf, RESOURCE_MIME_TYPE},
    http::{
        read_pending_body, read_request_head_checked, write_response, EventStream, HttpLimits,
        HttpMethod, HttpRequest, HttpResponse,
    },
    jsonrpc::{
        encode_error, encode_notification, encode_result, parse_message, JsonRpcError,
        JsonRpcMessage, JsonRpcRequest, INTERNAL_ERROR,
    },
    oauth::OAuthResource,
    session::{SessionError, SessionEvents, SessionLimits, SessionRegistry, SessionView},
};

/// The MCP revision this server implements. Everything in this module is
/// written against it: one endpoint, no JSON-RPC batching, a session id header,
/// and a protocol-version header on every request after initialization.
pub const PROTOCOL_VERSION: &str = "2025-06-18";

/// The header carrying the session a request belongs to.
pub const SESSION_HEADER: &str = "mcp-session-id";
/// The header carrying the negotiated revision.
pub const PROTOCOL_VERSION_HEADER: &str = "mcp-protocol-version";

/// MCP's own reserved code for a resource that does not exist.
pub const RESOURCE_NOT_FOUND: i32 = -32002;

/// Standard resource URI prefix for submitted server administration operations.
pub const OPERATION_URI_PREFIX: &str = "bbb://server/operation/";

/// Standard resource URI prefix for submitted client control operations.
pub const CLIENT_OPERATION_URI_PREFIX: &str = "bbb://client/operation/";

/// RFC 9728's discovery path for a protected resource's own metadata.
pub const PROTECTED_RESOURCE_METADATA_PATH: &str = "/.well-known/oauth-protected-resource";

const CORS_ALLOW_METHODS: &str = "GET, POST, DELETE, OPTIONS";
const CORS_ALLOW_HEADERS: &str =
    "Authorization, Content-Type, Accept, Mcp-Session-Id, Mcp-Protocol-Version, Last-Event-ID";
const CORS_EXPOSE_HEADERS: &str = "Mcp-Session-Id";

/// What protects this endpoint once it is reachable from off the host.
///
/// The default is the only shape that needs nothing declared, and it is the
/// only one that may not leave the loopback interface. An authenticated gateway
/// is an accepted deployment, but it terminates in front of a loopback bind
/// rather than being a third variant here: a variant that trusted a forwarded
/// header would be an unauthenticated socket with a promise attached.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum RemoteAuthority {
    #[default]
    LoopbackOnly,
    /// TLS terminated here, in front of an OAuth 2.1 protected resource. Both
    /// halves are required together: encryption without token validation would
    /// protect the wire and nothing else.
    Tls {
        material: crate::tls::TlsMaterial,
        oauth: crate::oauth::OAuthResource,
    },
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BindRefusal {
    #[error(
        "{0} is not a loopback address; a remote management endpoint needs TLS and an OAuth 2.1 \
         protected resource, or an authenticated gateway in front of a loopback bind"
    )]
    RemoteWithoutAuthority(SocketAddr),
    #[error("the endpoint declares TLS and OAuth but its auth configuration carries no resource")]
    AuthorityNotWired,
}

/// Whether this address may be bound under this authority.
///
/// Called before the listener is created, so a server that would have been
/// reachable without protection never binds at all.
pub fn check_bind<L: ScopeLadder>(
    address: SocketAddr,
    authority: &RemoteAuthority,
    auth: &AuthConfig<L>,
) -> Result<(), BindRefusal> {
    match authority {
        RemoteAuthority::LoopbackOnly if !address.ip().is_loopback() => {
            Err(BindRefusal::RemoteWithoutAuthority(address))
        }
        RemoteAuthority::Tls { oauth, .. } => {
            if auth.oauth.as_ref() != Some(oauth) {
                return Err(BindRefusal::AuthorityNotWired);
            }
            Ok(())
        }
        RemoteAuthority::LoopbackOnly => Ok(()),
    }
}

/// The endpoint's configuration. The identity fields default to what the host
/// calls itself; everything else is transport policy.
pub struct McpServerConfig<H: McpHost> {
    pub endpoint_path: String,
    pub server_name: String,
    pub server_title: String,
    pub server_version: String,
    pub instructions: Option<String>,
    /// The uri prefix under which one submitted operation's own record is
    /// readable. A call that outlived its deadline is still applied, so its
    /// answer has to remain reachable by id rather than being lost with the
    /// request.
    pub operation_uri_prefix: String,
    pub http: HttpLimits,
    pub sessions: SessionLimits,
    pub auth: AuthConfig<H::Scopes>,
    /// How long a caller waits for the answer before it is left to the
    /// operation record.
    pub request_timeout: Duration,
    /// Maximum elapsed time for a full HTTP request read (including idle
    /// keep-alive time), TLS handshake, response or individual SSE frame write,
    /// and transport shutdown. Waiting for host results or SSE events uses
    /// its own policy instead.
    pub transport_timeout: Duration,
    /// How often an idle event stream writes a comment frame.
    pub keep_alive_interval: Duration,
    pub max_connections: usize,
    /// How many finished operations stay readable by id.
    pub max_operation_records: usize,
}

impl<H: McpHost> Default for McpServerConfig<H> {
    fn default() -> Self {
        let identity = H::identity();
        Self {
            endpoint_path: "/mcp".to_string(),
            server_name: identity.name.to_string(),
            server_title: identity.title.to_string(),
            server_version: identity.version.to_string(),
            instructions: identity.instructions.map(str::to_string),
            operation_uri_prefix: identity.operation_uri_prefix.to_string(),
            http: HttpLimits::default(),
            sessions: SessionLimits::default(),
            auth: AuthConfig::default(),
            request_timeout: Duration::from_secs(10),
            transport_timeout: Duration::from_secs(10),
            keep_alive_interval: Duration::from_secs(15),
            max_connections: 16,
            max_operation_records: 256,
        }
    }
}

impl<H: McpHost> Clone for McpServerConfig<H> {
    fn clone(&self) -> Self {
        Self {
            endpoint_path: self.endpoint_path.clone(),
            server_name: self.server_name.clone(),
            server_title: self.server_title.clone(),
            server_version: self.server_version.clone(),
            instructions: self.instructions.clone(),
            operation_uri_prefix: self.operation_uri_prefix.clone(),
            http: self.http,
            sessions: self.sessions,
            auth: self.auth.clone(),
            request_timeout: self.request_timeout,
            transport_timeout: self.transport_timeout,
            keep_alive_interval: self.keep_alive_interval,
            max_connections: self.max_connections,
            max_operation_records: self.max_operation_records,
        }
    }
}

impl<H: McpHost> PartialEq for McpServerConfig<H> {
    fn eq(&self, other: &Self) -> bool {
        self.endpoint_path == other.endpoint_path
            && self.server_name == other.server_name
            && self.server_title == other.server_title
            && self.server_version == other.server_version
            && self.instructions == other.instructions
            && self.operation_uri_prefix == other.operation_uri_prefix
            && self.http == other.http
            && self.sessions == other.sessions
            && self.auth == other.auth
            && self.request_timeout == other.request_timeout
            && self.transport_timeout == other.transport_timeout
            && self.keep_alive_interval == other.keep_alive_interval
            && self.max_connections == other.max_connections
            && self.max_operation_records == other.max_operation_records
    }
}

impl<H: McpHost> Eq for McpServerConfig<H> {}

impl<H: McpHost> std::fmt::Debug for McpServerConfig<H> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("McpServerConfig")
            .field("endpoint_path", &self.endpoint_path)
            .field("server_name", &self.server_name)
            .field("server_title", &self.server_title)
            .field("server_version", &self.server_version)
            .field("instructions", &self.instructions)
            .field("operation_uri_prefix", &self.operation_uri_prefix)
            .field("http", &self.http)
            .field("sessions", &self.sessions)
            .field("auth", &self.auth)
            .field("request_timeout", &self.request_timeout)
            .field("transport_timeout", &self.transport_timeout)
            .field("keep_alive_interval", &self.keep_alive_interval)
            .field("max_connections", &self.max_connections)
            .field("max_operation_records", &self.max_operation_records)
            .finish()
    }
}

/// A submitted operation and the host's record projection of its answer.
#[derive(Debug, Clone)]
struct OperationRecord {
    operation_id: u64,
    principal: String,
    request_id: String,
    action: &'static str,
    outcome: Option<Value>,
}

/// Why an operation did not answer inside the caller's deadline.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ApplyFailure {
    Refused(CallRefusal),
    /// The host has not answered yet. The work is not cancelled, and the
    /// record is readable under the operation uri prefix.
    Deadline {
        operation_id: u64,
    },
    /// The host dropped the answer, which only happens on shutdown.
    Abandoned {
        operation_id: u64,
    },
}

const AUTH_FAILURE_LIMIT_PER_WINDOW: usize = 30;
const AUTH_FAILURE_WINDOW: Duration = Duration::from_secs(60);
const MAX_TRACKED_AUTH_IPS: usize = 1024;

#[derive(Debug, Default)]
struct AuthRateLimiter {
    entries: HashMap<IpAddr, VecDeque<Instant>>,
}

impl AuthRateLimiter {
    fn is_limited(&mut self, ip: IpAddr, now: Instant) -> Option<u64> {
        let cutoff = now.checked_sub(AUTH_FAILURE_WINDOW).unwrap_or(now);
        let timestamps = self.entries.get_mut(&ip)?;
        while timestamps.front().is_some_and(|&ts| ts <= cutoff) {
            timestamps.pop_front();
        }
        if timestamps.is_empty() {
            self.entries.remove(&ip);
            return None;
        }
        if timestamps.len() >= AUTH_FAILURE_LIMIT_PER_WINDOW {
            let oldest = *timestamps.front()?;
            let wait = AUTH_FAILURE_WINDOW
                .saturating_sub(now.saturating_duration_since(oldest))
                .as_secs()
                .max(1);
            Some(wait)
        } else {
            None
        }
    }

    fn record_failure(&mut self, ip: IpAddr, now: Instant) {
        let cutoff = now.checked_sub(AUTH_FAILURE_WINDOW).unwrap_or(now);
        if !self.entries.contains_key(&ip) && self.entries.len() >= MAX_TRACKED_AUTH_IPS {
            self.entries.retain(|_, timestamps| {
                while timestamps.front().is_some_and(|&ts| ts <= cutoff) {
                    timestamps.pop_front();
                }
                !timestamps.is_empty()
            });
            if self.entries.len() >= MAX_TRACKED_AUTH_IPS {
                if let Some(oldest_ip) = self
                    .entries
                    .iter()
                    .min_by_key(|(_, timestamps)| timestamps.back().copied())
                    .map(|(&addr, _)| addr)
                {
                    self.entries.remove(&oldest_ip);
                }
            }
        }
        let timestamps = self.entries.entry(ip).or_default();
        while timestamps.front().is_some_and(|&ts| ts <= cutoff) {
            timestamps.pop_front();
        }
        timestamps.push_back(now);
    }
}

pub struct McpServer<H: McpHost> {
    config: McpServerConfig<H>,
    oauth_resource: Option<Arc<OAuthResource>>,
    host: H,
    sessions: SessionRegistry<H::Scopes>,
    sse_permits: Arc<Semaphore>,
    operations: Mutex<VecDeque<OperationRecord>>,
    auth_rate_limiter: Mutex<AuthRateLimiter>,
}

impl<H: McpHost> McpServer<H> {
    pub fn new(config: McpServerConfig<H>, host: impl Into<H>) -> Arc<Self> {
        let sessions = SessionRegistry::new(config.sessions);
        let sse_permits = Arc::new(Semaphore::new(config.sessions.max_sessions));
        let oauth_resource = config.auth.oauth.clone().map(Arc::new);
        Arc::new(Self {
            config,
            oauth_resource,
            host: host.into(),
            sessions,
            sse_permits,
            operations: Mutex::new(VecDeque::new()),
            auth_rate_limiter: Mutex::new(AuthRateLimiter::default()),
        })
    }

    pub fn config(&self) -> &McpServerConfig<H> {
        &self.config
    }

    pub fn host(&self) -> &H {
        &self.host
    }

    pub fn sessions(&self) -> &SessionRegistry<H::Scopes> {
        &self.sessions
    }

    /// Both RFC 9728 forms: the bare well-known path, and the one that carries
    /// this endpoint's own path so one host can protect several resources.
    fn is_metadata_path(&self, path: &str) -> bool {
        path == PROTECTED_RESOURCE_METADATA_PATH
            || path
                == format!(
                    "{PROTECTED_RESOURCE_METADATA_PATH}{}",
                    self.config.endpoint_path
                )
    }

    /// Turns the host's change stream into the subscription notifications
    /// each session asked for.
    pub fn spawn_change_fanout(self: &Arc<Self>) -> tokio::task::JoinHandle<()> {
        let server = Arc::clone(self);
        let mut changes = self.host.changes();
        tokio::spawn(async move {
            loop {
                match changes.recv().await {
                    Ok(change) => {
                        let message = encode_notification(
                            "notifications/resources/updated",
                            json!({ "uri": change.uri }),
                        );
                        server.sessions.publish(change.uri, &message);
                    }
                    // A lagged receiver missed changes it can no longer name, so
                    // every subscriber is told its resource may have moved.
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        for resource in server.host.resources() {
                            let message = encode_notification(
                                "notifications/resources/updated",
                                json!({ "uri": resource.uri }),
                            );
                            server.sessions.publish(resource.uri, &message);
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
                }
            }
        })
    }

    pub async fn serve(
        self: Arc<Self>,
        listener: TcpListener,
        shutdown: impl Future<Output = ()> + Send,
    ) {
        let fanout = self.spawn_change_fanout();
        let connections = Arc::new(Semaphore::new(self.config.max_connections));
        tokio::pin!(shutdown);
        loop {
            tokio::select! {
                _ = &mut shutdown => break,
                accepted = listener.accept() => {
                    let Ok((stream, peer)) = accepted else {
                        tokio::select! {
                            _ = &mut shutdown => break,
                            _ = tokio::time::sleep(Duration::from_millis(50)) => continue,
                        }
                    };
                    let _ = stream.set_nodelay(true);
                    let Ok(permit) = Arc::clone(&connections).try_acquire_owned() else {
                        // Refusing to accept is the only honest answer once the
                        // connection budget is spent; a queued socket would look
                        // like progress that is not happening.
                        drop(stream);
                        continue;
                    };
                    let server = Arc::clone(&self);
                    tokio::spawn(async move {
                        server
                            .handle_connection_with_permit(stream, peer, Some(permit))
                            .await;
                    });
                }
            }
        }
        fanout.abort();
    }

    /// The same accept loop, terminating TLS before the first byte is read.
    pub async fn serve_tls(
        self: Arc<Self>,
        listener: TcpListener,
        acceptor: tokio_rustls::TlsAcceptor,
        shutdown: impl Future<Output = ()> + Send,
    ) {
        let fanout = self.spawn_change_fanout();
        let connections = Arc::new(Semaphore::new(self.config.max_connections));
        tokio::pin!(shutdown);
        loop {
            tokio::select! {
                _ = &mut shutdown => break,
                accepted = listener.accept() => {
                    let Ok((stream, peer)) = accepted else {
                        tokio::select! {
                            _ = &mut shutdown => break,
                            _ = tokio::time::sleep(Duration::from_millis(50)) => continue,
                        }
                    };
                    let _ = stream.set_nodelay(true);
                    let Ok(permit) = Arc::clone(&connections).try_acquire_owned() else {
                        drop(stream);
                        continue;
                    };
                    let server = Arc::clone(&self);
                    let acceptor = acceptor.clone();
                    tokio::spawn(async move {
                        // A handshake that never completes must not hold a
                        // connection permit for as long as the peer likes.
                        if let Ok(Ok(stream)) =
                            timeout(server.config.transport_timeout, acceptor.accept(stream)).await
                        {
                            server
                                .handle_connection_with_permit(stream, peer, Some(permit))
                                .await;
                        }
                    });
                }
            }
        }
        fanout.abort();
    }

    pub async fn handle_connection<S>(self: Arc<Self>, stream: S, peer: SocketAddr)
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
    {
        self.handle_connection_with_permit(stream, peer, None).await;
    }

    async fn handle_connection_with_permit<S>(
        self: Arc<Self>,
        stream: S,
        peer: SocketAddr,
        mut connection_permit: Option<OwnedSemaphorePermit>,
    ) where
        S: AsyncRead + AsyncWrite + Unpin + Send,
    {
        let (reader, writer) = tokio::io::split(stream);
        let mut reader = BufReader::new(reader);
        let mut writer = writer;
        loop {
            let deadline = tokio::time::Instant::now() + self.config.transport_timeout;
            let pending = match tokio::time::timeout_at(
                deadline,
                read_request_head_checked(&mut reader, &self.config.http),
            )
            .await
            {
                Ok(Ok(Some(pending))) => pending,
                Ok(Ok(None)) | Err(_) => break,
                Ok(Err(error)) => {
                    let response = HttpResponse::text(error.status(), &error.to_string());
                    let _ = self
                        .transport_io(write_response(&mut writer, &response, false, true))
                        .await;
                    break;
                }
            };
            let head_only = pending.request.method == HttpMethod::Head;
            let unread_body = pending.has_body();
            let allowed_origin = self
                .matched_allowed_origin(&pending.request)
                .map(str::to_owned);
            let principal = match self.precheck_request(&pending.request, peer).await {
                Ok(principal) => principal,
                Err(response) => {
                    let response = apply_cors_headers(response, allowed_origin.as_deref());
                    // Do not reuse a connection whose declared body was never drained.
                    let keep_alive = pending.request.keep_alive && !unread_body;
                    if self
                        .transport_io(write_response(
                            &mut writer,
                            &response,
                            keep_alive,
                            !head_only,
                        ))
                        .await
                        .is_err()
                        || !keep_alive
                    {
                        break;
                    }
                    continue;
                }
            };
            let request = match tokio::time::timeout_at(
                deadline,
                read_pending_body(pending, &mut reader, &mut writer),
            )
            .await
            {
                Ok(Ok(request)) => request,
                Ok(Err(error)) => {
                    let response = apply_cors_headers(
                        HttpResponse::text(error.status(), &error.to_string()),
                        allowed_origin.as_deref(),
                    );
                    let _ = self
                        .transport_io(write_response(&mut writer, &response, false, true))
                        .await;
                    break;
                }
                Err(_) => break,
            };
            let keep_alive = request.keep_alive;
            match self.route_authenticated(&request, &principal).await {
                Routed::Answer(response) => {
                    let response = apply_cors_headers(response, allowed_origin.as_deref());
                    if self
                        .transport_io(write_response(
                            &mut writer,
                            &response,
                            keep_alive,
                            !head_only,
                        ))
                        .await
                        .is_err()
                        || !keep_alive
                    {
                        break;
                    }
                }
                Routed::Stream { session } => {
                    let Ok(sse_permit) = Arc::clone(&self.sse_permits).try_acquire_owned() else {
                        let response = apply_cors_headers(
                            HttpResponse::text(
                                503,
                                "this server holds as many active event streams as it may",
                            ),
                            allowed_origin.as_deref(),
                        );
                        let _ = self
                            .transport_io(write_response(&mut writer, &response, false, true))
                            .await;
                        break;
                    };
                    let events = match self.sessions.open_stream(&session) {
                        Ok(events) => events,
                        Err(refusal) => {
                            drop(sse_permit);
                            let response = apply_cors_headers(
                                stream_open_error_response(refusal),
                                allowed_origin.as_deref(),
                            );
                            let _ = self
                                .transport_io(write_response(&mut writer, &response, false, true))
                                .await;
                            break;
                        }
                    };
                    // Release the short-lived RPC connection permit once the SSE stream
                    // acquires its own concurrency permit so `watch` streams cannot starve RPCs.
                    drop(connection_permit.take());
                    self.run_event_stream(
                        reader,
                        writer,
                        &session,
                        allowed_origin.as_deref(),
                        events,
                    )
                    .await;
                    drop(sse_permit);
                    return;
                }
            }
        }
        // A transport that can say goodbye should: over TLS this is the
        // close_notify that tells the peer the stream ended rather than broke.
        let _ = self.transport_io(writer.shutdown()).await;
    }

    async fn transport_io<T>(
        &self,
        operation: impl Future<Output = io::Result<T>>,
    ) -> io::Result<T> {
        timeout(self.config.transport_timeout, operation)
            .await
            .map_err(|_| {
                io::Error::new(io::ErrorKind::TimedOut, "MCP transport deadline elapsed")
            })?
    }

    fn matched_allowed_origin<'a>(&self, request: &'a HttpRequest) -> Option<&'a str> {
        let origin = request.headers.get("origin")?;
        origin_allowed(&self.config.auth, &request.headers).then_some(origin)
    }

    async fn precheck_request(
        &self,
        request: &HttpRequest,
        peer: SocketAddr,
    ) -> Result<Principal<H::Scopes>, HttpResponse> {
        if self.is_metadata_path(&request.path) {
            // Discovery metadata is what an unauthenticated client reads to
            // learn how to authenticate, so it is served before the check.
            return Err(match &self.config.auth.oauth {
                Some(resource) => json_response(200, &resource.metadata_document()),
                None => HttpResponse::text(404, "this endpoint is not an OAuth protected resource"),
            });
        }
        if request.path != self.config.endpoint_path {
            return Err(HttpResponse::text(404, "no such endpoint"));
        }
        if !origin_allowed(&self.config.auth, &request.headers) {
            return Err(HttpResponse::text(403, "this origin is not accepted"));
        }
        if request.method == HttpMethod::Options {
            let mut response = HttpResponse::new(204).with_header("allow", CORS_ALLOW_METHODS);
            if self.matched_allowed_origin(request).is_some() {
                response = response
                    .with_header("access-control-allow-methods", CORS_ALLOW_METHODS)
                    .with_header("access-control-allow-headers", CORS_ALLOW_HEADERS);
            }
            return Err(response);
        }
        if let Some(retry_after) = self
            .auth_rate_limiter
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_limited(peer.ip(), Instant::now())
        {
            return Err(
                HttpResponse::text(429, "too many authentication failures; retry later")
                    .with_header("retry-after", retry_after.to_string()),
            );
        }
        let principal = authenticate_async_with_oauth(
            &self.config.auth,
            self.oauth_resource.as_ref(),
            &request.headers,
            peer,
            self.host.now(),
        )
        .await
        .map_err(|failure| {
            if matches!(
                failure,
                AuthFailure::MissingCredential | AuthFailure::InvalidCredential
            ) {
                self.auth_rate_limiter
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .record_failure(peer.ip(), Instant::now());
            }
            HttpResponse::text(failure.status(), failure.detail())
                .with_header("www-authenticate", challenge(&self.config.auth, &failure))
        })?;
        match request.method {
            HttpMethod::Post => {
                if let Some(error) = validate_post_headers(request) {
                    return Err(error);
                }
                Ok(principal)
            }
            HttpMethod::Get | HttpMethod::Delete => Ok(principal),
            _ => Err(
                HttpResponse::text(405, "the MCP endpoint accepts GET, POST and DELETE")
                    .with_header("allow", "GET, POST, DELETE, OPTIONS"),
            ),
        }
    }

    async fn route_authenticated(
        self: &Arc<Self>,
        request: &HttpRequest,
        principal: &Principal<H::Scopes>,
    ) -> Routed {
        match request.method {
            HttpMethod::Post => Routed::Answer(self.handle_post(request, principal).await),
            HttpMethod::Get => self.handle_get(request, principal),
            HttpMethod::Delete => Routed::Answer(self.handle_delete(request, principal)),
            _ => Routed::Answer(
                HttpResponse::text(405, "the MCP endpoint accepts GET, POST and DELETE")
                    .with_header("allow", "GET, POST, DELETE, OPTIONS"),
            ),
        }
    }

    async fn handle_post(
        self: &Arc<Self>,
        request: &HttpRequest,
        principal: &Principal<H::Scopes>,
    ) -> HttpResponse {
        if let Some(error) = validate_post_headers(request) {
            return error;
        }
        let message = match parse_message(&request.body) {
            Ok(message) => message,
            Err(error) => return json_response(400, &encode_error(None, &error)),
        };
        let call = match message {
            // An answer or a notification needs no reply of its own; the
            // notification this server acts on is `notifications/initialized`,
            // which only marks the handshake complete.
            JsonRpcMessage::Answer(_) => {
                if let Some(session_id) = request.headers.get(SESSION_HEADER) {
                    let Some(session) = self.sessions.view(session_id) else {
                        return HttpResponse::text(404, "this session is unknown or was ended");
                    };
                    if session.principal != *principal {
                        return HttpResponse::text(
                            403,
                            "this session belongs to another principal",
                        );
                    }
                    self.sessions.touch(session_id);
                }
                return HttpResponse::new(202);
            }
            JsonRpcMessage::Notification(notification) => {
                if let Some(session_id) = request.headers.get(SESSION_HEADER) {
                    let Some(session) = self.sessions.view(session_id) else {
                        return HttpResponse::text(404, "this session is unknown or was ended");
                    };
                    if session.principal != *principal {
                        return HttpResponse::text(
                            403,
                            "this session belongs to another principal",
                        );
                    }
                    if let Some(declared) = request.headers.get(PROTOCOL_VERSION_HEADER) {
                        if declared != session.protocol_version {
                            return HttpResponse::text(
                                400,
                                &format!(
                                    "this session speaks {}, not {declared}",
                                    session.protocol_version
                                ),
                            );
                        }
                    }
                    if notification.method == "notifications/initialized" {
                        self.sessions.mark_initialized(session_id);
                    } else {
                        self.sessions.touch(session_id);
                    }
                }
                return HttpResponse::new(202);
            }
            JsonRpcMessage::Request(call) => call,
        };

        if call.method == "initialize" {
            return self.handle_initialize(&call, principal);
        }

        let Some(session_id) = request.headers.get(SESSION_HEADER) else {
            return json_response(
                400,
                &encode_error(
                    Some(&call.id),
                    &JsonRpcError::invalid_request(format!(
                        "every request after initialize carries {SESSION_HEADER}"
                    )),
                ),
            );
        };
        let Some(session) = self.sessions.view(session_id) else {
            return json_response(
                404,
                &encode_error(
                    Some(&call.id),
                    &JsonRpcError::invalid_request("this session is unknown or was ended"),
                ),
            );
        };
        if session.principal != *principal {
            return HttpResponse::text(403, "this session belongs to another principal");
        }
        if let Some(declared) = request.headers.get(PROTOCOL_VERSION_HEADER) {
            if declared != session.protocol_version {
                return json_response(
                    400,
                    &encode_error(
                        Some(&call.id),
                        &JsonRpcError::invalid_request(format!(
                            "this session speaks {}, not {declared}",
                            session.protocol_version
                        )),
                    ),
                );
            }
        }
        if !session.initialized {
            return json_response(
                400,
                &encode_error(
                    Some(&call.id),
                    &JsonRpcError::invalid_request(
                        "this session has not completed initialization; send notifications/initialized first",
                    ),
                ),
            );
        }
        let _inflight = match self.sessions.enter_request(&session.id) {
            Ok(guard) => guard,
            Err(SessionError::NotInitialized) => {
                return json_response(
                    400,
                    &encode_error(
                        Some(&call.id),
                        &JsonRpcError::invalid_request(
                            "this session has not completed initialization; send notifications/initialized first",
                        ),
                    ),
                );
            }
            Err(_) => {
                return json_response(
                    404,
                    &encode_error(
                        Some(&call.id),
                        &JsonRpcError::invalid_request("this session is unknown or was ended"),
                    ),
                );
            }
        };

        let id = call.id.clone();
        match self.dispatch(&session, call).await {
            Ok(result) => json_response(200, &encode_result(&id, result)),
            Err(error) => json_response(200, &encode_error(Some(&id), &error)),
        }
    }

    fn handle_initialize(
        &self,
        call: &JsonRpcRequest,
        principal: &Principal<H::Scopes>,
    ) -> HttpResponse {
        let requested = call
            .params
            .get("protocolVersion")
            .and_then(Value::as_str)
            .unwrap_or(PROTOCOL_VERSION);
        let session_id = new_session_id();
        if self
            .sessions
            .create(session_id.clone(), principal.clone(), PROTOCOL_VERSION)
            .is_err()
        {
            return json_response(
                503,
                &encode_error(
                    Some(&call.id),
                    &JsonRpcError::new(INTERNAL_ERROR, "this server holds no more sessions"),
                ),
            );
        }
        // The answer always names the revision this server implements. A client
        // that asked for another one learns immediately, rather than after a
        // message it could not have understood.
        let result = json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {
                "tools": { "listChanged": false },
                "resources": { "subscribe": true, "listChanged": false },
            },
            "serverInfo": {
                "name": self.config.server_name,
                "title": self.config.server_title,
                "version": self.config.server_version,
            },
            "instructions": self.config.instructions,
            "_meta": {
                "bbb/scope": H::Scopes::name(principal.scope),
                "bbb/principal": principal.name,
                "bbb/requestedProtocolVersion": requested,
            },
        });
        json_response(200, &encode_result(&call.id, result)).with_header(SESSION_HEADER, session_id)
    }

    fn handle_get(&self, request: &HttpRequest, principal: &Principal<H::Scopes>) -> Routed {
        if !request.headers.accepts("text/event-stream") {
            return Routed::Answer(HttpResponse::text(
                406,
                "the event stream is text/event-stream",
            ));
        }
        let Some(session_id) = request.headers.get(SESSION_HEADER) else {
            return Routed::Answer(HttpResponse::text(400, "an event stream names its session"));
        };
        match self.sessions.view(session_id) {
            Some(session) if session.principal != *principal => Routed::Answer(HttpResponse::text(
                403,
                "this session belongs to another principal",
            )),
            Some(session) if !session.initialized => Routed::Answer(HttpResponse::text(
                400,
                "this session has not completed initialization; send notifications/initialized first",
            )),
            Some(session) => {
                if let Some(declared) = request.headers.get(PROTOCOL_VERSION_HEADER) {
                    if declared != session.protocol_version {
                        return Routed::Answer(HttpResponse::text(
                            400,
                            &format!(
                                "this session speaks {}, not {declared}",
                                session.protocol_version
                            ),
                        ));
                    }
                }
                Routed::Stream {
                    session: session_id.to_string(),
                }
            }
            None => Routed::Answer(HttpResponse::text(
                404,
                "this session is unknown or was ended",
            )),
        }
    }

    fn handle_delete(
        &self,
        request: &HttpRequest,
        principal: &Principal<H::Scopes>,
    ) -> HttpResponse {
        let Some(session_id) = request.headers.get(SESSION_HEADER) else {
            return HttpResponse::text(400, "ending a session names it");
        };
        match self.sessions.view(session_id) {
            Some(session) if session.principal != *principal => {
                HttpResponse::text(403, "this session belongs to another principal")
            }
            Some(_) => {
                self.sessions.remove(session_id);
                HttpResponse::new(204)
            }
            None => HttpResponse::text(404, "this session is unknown or was ended"),
        }
    }

    async fn run_event_stream<R, W>(
        &self,
        mut reader: R,
        writer: W,
        session_id: &str,
        allowed_origin: Option<&str>,
        events: Arc<SessionEvents>,
    ) where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        let mut headers = vec![(SESSION_HEADER.to_string(), session_id.to_string())];
        if let Some(origin) = allowed_origin {
            headers.push((
                "access-control-allow-origin".to_string(),
                origin.to_string(),
            ));
            headers.push(("vary".to_string(), "Origin".to_string()));
            headers.push((
                "access-control-expose-headers".to_string(),
                CORS_EXPOSE_HEADERS.to_string(),
            ));
        }
        let Ok(mut stream) = self
            .transport_io(EventStream::start(writer, &headers))
            .await
        else {
            self.sessions.close_stream(session_id);
            return;
        };
        let mut next_event_id = 1u64;
        'stream: loop {
            // Register the waiter before draining the queue and checking live/closed
            // state so neither a concurrent `push()` nor a concurrent `remove()` ->
            // `wake()` can be missed between the check and entering `select!`.
            let notified = events.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            let pending = events.take();
            if !pending.is_empty() {
                for message in pending {
                    let payload = message.to_string();
                    if self
                        .transport_io(stream.send(Some(next_event_id), &payload))
                        .await
                        .is_err()
                    {
                        break 'stream;
                    }
                    self.sessions.touch(session_id);
                    next_event_id += 1;
                }
                continue;
            }
            if events.is_closed() || !self.sessions.is_live(session_id) {
                break;
            }
            tokio::select! {
                () = wait_for_peer_close(&mut reader) => break,
                waited = timeout(self.config.keep_alive_interval, notified) => {
                    if waited.is_err() {
                        if self.transport_io(stream.keep_alive()).await.is_err() {
                            break;
                        }
                        self.sessions.touch(session_id);
                    }
                }
            }
        }
        self.sessions.close_stream(session_id);
        let _ = self.transport_io(stream.finish()).await;
    }

    async fn dispatch(
        self: &Arc<Self>,
        session: &SessionView<H::Scopes>,
        call: JsonRpcRequest,
    ) -> Result<Value, JsonRpcError> {
        match call.method.as_str() {
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({
                "tools": self.host.tools().iter().map(|tool| tool.to_value()).collect::<Vec<_>>(),
            })),
            "resources/list" => Ok(json!({
                "resources": self.host.resources().iter().map(|resource| resource.to_value()).collect::<Vec<_>>(),
            })),
            "resources/templates/list" => Ok(json!({
                "resourceTemplates": [{
                    "uriTemplate": format!("{}{{operationId}}", self.config.operation_uri_prefix),
                    "name": "operation",
                    "title": "Submitted operation",
                    "description": "The record of one submitted operation, readable by the principal that submitted it even after its request timed out.",
                    "mimeType": RESOURCE_MIME_TYPE,
                }],
            })),
            "tools/call" => self.call_tool(session, &call).await,
            "resources/read" => self.read_resource(session, &call).await,
            "resources/subscribe" => {
                let uri = resource_uri(&call)?;
                let resource = self
                    .host
                    .resource(&uri)
                    .ok_or_else(|| resource_not_found(&uri))?;
                self.sessions
                    .subscribe(&session.id, resource.uri)
                    .map_err(|err| match err {
                        SessionError::TooManySubscriptions => JsonRpcError::invalid_params(
                            "this session holds as many subscriptions as it may",
                        ),
                        SessionError::Unknown | SessionError::NotInitialized => {
                            JsonRpcError::invalid_request("this session was ended")
                        }
                        SessionError::TooManySessions | SessionError::StreamAlreadyOpen => {
                            JsonRpcError::invalid_request("this session cannot subscribe")
                        }
                    })?;
                Ok(json!({}))
            }
            "resources/unsubscribe" => {
                let uri = resource_uri(&call)?;
                self.sessions
                    .unsubscribe(&session.id, &uri)
                    .map_err(|_| JsonRpcError::invalid_request("this session was ended"))?;
                Ok(json!({}))
            }
            other => Err(JsonRpcError::method_not_found(other)),
        }
    }

    async fn call_tool(
        self: &Arc<Self>,
        session: &SessionView<H::Scopes>,
        call: &JsonRpcRequest,
    ) -> Result<Value, JsonRpcError> {
        let name = call
            .params
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params("a tool call names its tool"))?;
        let arguments = call
            .params
            .get("arguments")
            .cloned()
            .unwrap_or(Value::Object(Map::new()));
        let submitted = self
            .apply(session, call, |caller| {
                self.host.call_tool(caller, name, &arguments)
            })
            .await;
        match submitted {
            Ok(outcome) => Ok(self.host.tool_result(&outcome)),
            Err(ApplyFailure::Refused(CallRefusal::Unknown)) => Err(JsonRpcError::invalid_params(
                format!("{name:?} is not a tool"),
            )),
            Err(ApplyFailure::Refused(CallRefusal::InvalidArguments(detail))) => {
                Err(JsonRpcError::invalid_params(detail))
            }
            Err(ApplyFailure::Refused(CallRefusal::Unavailable { detail, retryable })) => {
                Err(JsonRpcError::new(INTERNAL_ERROR, detail)
                    .with_data(json!({ "retryable": retryable })))
            }
            Err(failure) => Ok(self.deferred_result(&failure)),
        }
    }

    async fn read_resource(
        self: &Arc<Self>,
        session: &SessionView<H::Scopes>,
        call: &JsonRpcRequest,
    ) -> Result<Value, JsonRpcError> {
        let uri = resource_uri(call)?;
        if let Some(id) = uri.strip_prefix(self.config.operation_uri_prefix.as_str()) {
            let operation_id = id.parse::<u64>().map_err(|_| resource_not_found(&uri))?;
            let record = self
                .operation_record(operation_id, &session.principal.name)
                .ok_or_else(|| resource_not_found(&uri))?;
            return Ok(contents(&uri, &record));
        }
        let submitted = self
            .apply(session, call, |caller| {
                self.host.read_resource(caller, &uri)
            })
            .await;
        match submitted {
            Ok(outcome) => self
                .host
                .resource_contents(&outcome)
                .map(|value| contents(&uri, &value)),
            Err(ApplyFailure::Refused(CallRefusal::Unknown)) => Err(resource_not_found(&uri)),
            Err(ApplyFailure::Refused(CallRefusal::InvalidArguments(detail))) => {
                Err(JsonRpcError::invalid_params(detail))
            }
            Err(ApplyFailure::Refused(CallRefusal::Unavailable { detail, .. })) => {
                Err(JsonRpcError::new(INTERNAL_ERROR, detail))
            }
            Err(failure) => Err(
                JsonRpcError::new(INTERNAL_ERROR, self.deferred_detail(&failure))
                    .with_data(self.deferred_data(&failure)),
            ),
        }
    }

    /// Submits one call and waits for the host to apply it.
    async fn apply(
        self: &Arc<Self>,
        session: &SessionView<H::Scopes>,
        call: &JsonRpcRequest,
        submit: impl FnOnce(&Caller<ScopeOf<H::Scopes>>) -> Result<PendingCall<H::Outcome>, CallRefusal>,
    ) -> Result<H::Outcome, ApplyFailure> {
        let caller = Caller {
            principal: session.principal.name.clone(),
            scope: session.principal.scope,
            request_id: idempotency_key(session, call),
        };
        let pending = submit(&caller).map_err(ApplyFailure::Refused)?;
        let operation_id = pending.operation_id;
        self.record_submitted(OperationRecord {
            operation_id,
            principal: caller.principal,
            request_id: caller.request_id,
            action: pending.action,
            outcome: None,
        });
        let mut answer = pending.answer;
        match timeout(self.config.request_timeout, &mut answer).await {
            Ok(Ok(outcome)) => {
                self.record_answer(operation_id, &outcome);
                Ok(outcome)
            }
            Ok(Err(_)) => Err(ApplyFailure::Abandoned { operation_id }),
            Err(_) => {
                // The deadline belongs to the caller, not to the work: the
                // request is already submitted and the host will apply it. So
                // the receiver outlives this call and finishes the record, and
                // the caller is told where to read the answer.
                let server = Arc::clone(self);
                tokio::spawn(async move {
                    if let Ok(outcome) = answer.await {
                        server.record_answer(operation_id, &outcome);
                    }
                });
                Err(ApplyFailure::Deadline { operation_id })
            }
        }
    }

    fn record_submitted(&self, record: OperationRecord) {
        if self.config.max_operation_records == 0 {
            return;
        }
        let mut operations = self
            .operations
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        while operations.len() >= self.config.max_operation_records {
            operations.pop_front();
        }
        operations.push_back(record);
    }

    fn record_answer(&self, operation_id: u64, outcome: &H::Outcome) {
        let outcome = self.host.outcome_record(outcome);
        let mut operations = self
            .operations
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(record) = operations
            .iter_mut()
            .find(|record| record.operation_id == operation_id)
        {
            record.outcome = Some(outcome);
        }
    }

    /// The record of one operation, readable only by the principal that
    /// submitted it.
    fn operation_record(&self, operation_id: u64, principal: &str) -> Option<Value> {
        let operations = self
            .operations
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let record = operations
            .iter()
            .find(|record| record.operation_id == operation_id && record.principal == principal)?;
        Some(json!({
            "operationId": record.operation_id,
            "requestId": record.request_id,
            "action": record.action,
            "state": if record.outcome.is_some() { "applied" } else { "pending" },
            "outcome": record.outcome,
        }))
    }

    fn deferred_detail(&self, failure: &ApplyFailure) -> String {
        match failure {
            ApplyFailure::Deadline { operation_id } => format!(
                "operation {operation_id} did not answer in time; it is still applying and its \
                 record is at {}{operation_id}",
                self.config.operation_uri_prefix
            ),
            ApplyFailure::Abandoned { operation_id } => {
                format!("operation {operation_id} was abandoned by a stopping server")
            }
            ApplyFailure::Refused(CallRefusal::Unavailable { detail, .. }) => detail.clone(),
            ApplyFailure::Refused(CallRefusal::InvalidArguments(detail)) => detail.clone(),
            ApplyFailure::Refused(CallRefusal::Unknown) => "unknown call".to_string(),
        }
    }

    fn deferred_data(&self, failure: &ApplyFailure) -> Value {
        match failure {
            ApplyFailure::Deadline { operation_id } | ApplyFailure::Abandoned { operation_id } => {
                json!({
                    "operationId": operation_id,
                    "operationUri": format!("{}{operation_id}", self.config.operation_uri_prefix),
                })
            }
            ApplyFailure::Refused(_) => Value::Null,
        }
    }

    fn deferred_result(&self, failure: &ApplyFailure) -> Value {
        json!({
            "content": [{ "type": "text", "text": self.deferred_detail(failure) }],
            "isError": true,
            "_meta": { "bbb/deferred": self.deferred_data(failure) },
        })
    }
}

enum Routed {
    Answer(HttpResponse),
    Stream { session: String },
}

fn apply_cors_headers(response: HttpResponse, allowed_origin: Option<&str>) -> HttpResponse {
    let Some(origin) = allowed_origin else {
        return response;
    };
    response
        .with_header("access-control-allow-origin", origin)
        .with_header("vary", "Origin")
        .with_header("access-control-expose-headers", CORS_EXPOSE_HEADERS)
}

fn validate_post_headers(request: &HttpRequest) -> Option<HttpResponse> {
    if !request.headers.accepts("application/json") {
        return Some(HttpResponse::text(
            406,
            "this endpoint answers application/json",
        ));
    }
    if let Some(content_type) = request.headers.get("content-type") {
        let media_type = content_type
            .split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        if media_type != "application/json" {
            return Some(HttpResponse::text(
                415,
                "a request body is application/json",
            ));
        }
    }
    None
}

fn stream_open_error_response(refusal: SessionError) -> HttpResponse {
    match refusal {
        SessionError::NotInitialized => HttpResponse::text(
            400,
            "this session has not completed initialization; send notifications/initialized first",
        ),
        SessionError::StreamAlreadyOpen => {
            HttpResponse::text(409, "this session already has an event stream")
        }
        _ => HttpResponse::text(404, "this session is unknown or was ended"),
    }
}

fn json_response(status: u16, body: &Value) -> HttpResponse {
    HttpResponse::json(status, body.to_string().into_bytes())
}

fn contents(uri: &str, value: &Value) -> Value {
    json!({
        "contents": [{
            "uri": uri,
            "mimeType": RESOURCE_MIME_TYPE,
            "text": serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string()),
        }],
    })
}

fn resource_uri(call: &JsonRpcRequest) -> Result<String, JsonRpcError> {
    call.params
        .get("uri")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| JsonRpcError::invalid_params("a resource request names its uri"))
}

fn resource_not_found(uri: &str) -> JsonRpcError {
    JsonRpcError::new(RESOURCE_NOT_FOUND, format!("{uri} is not a resource"))
        .with_data(json!({ "uri": uri }))
}

/// The idempotency key one call carries.
///
/// A client that wants a retry to be recognised as the same operation supplies
/// its own key in `_meta`; otherwise the session and the JSON-RPC id name it,
/// which makes a retry on the same connection idempotent and a retry after a
/// reconnect a new operation.
fn idempotency_key<L: ScopeLadder>(session: &SessionView<L>, call: &JsonRpcRequest) -> String {
    call.params
        .get("_meta")
        .and_then(|meta| meta.get("bbb/requestId"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| format!("{}:{}", session.id, call.id.as_text()))
}

/// A session id with 128 bits of entropy, in the visible-ASCII form the
/// protocol requires of the header.
fn new_session_id() -> String {
    use ring::rand::{SecureRandom, SystemRandom};

    let mut bytes = [0u8; 16];
    SystemRandom::new()
        .fill(&mut bytes)
        .expect("session IDs require OS randomness");
    format!("{:032x}", u128::from_be_bytes(bytes))
}

async fn wait_for_peer_close<R: AsyncRead + Unpin>(reader: &mut R) {
    let mut scratch = [0u8; 64];
    loop {
        match reader.read(&mut scratch).await {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
    }
}
