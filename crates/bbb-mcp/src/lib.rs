//! The MCP Streamable HTTP transport bbb's client and server share.
//!
//! Vanilla 26.1 reaches administration over a WebSocket JSON-RPC management
//! server (`net.minecraft.server.jsonrpc`); bbb speaks MCP Streamable HTTP
//! instead, pinned to one protocol revision. This crate owns only what the
//! wire needs: HTTP framing, JSON-RPC envelopes, sessions and version
//! negotiation, authentication, TLS, the event stream, deadlines and the
//! operation records. What the tools and resources mean is a [`McpHost`]
//! implementation, and every host is served the same way.

mod auth;
pub mod control;
mod host;
pub mod http;
mod jsonrpc;
mod oauth;
mod server;
mod session;
mod tls;

pub use auth::{
    authenticate, challenge, origin_allowed, AuthConfig, AuthFailure, Principal, TokenGrant,
    LOOPBACK_PRINCIPAL,
};
pub use control::{
    ControlRequest, ControlResponse, ControlScreenshot, ControlTunnelClientMessage,
    ControlTunnelHostMessage, ControlTunnelScreenshot, ControlWireError, RemoteControlAnswer,
    RemoteControlEndpoint,
};
pub use host::{
    CallRefusal, Caller, HostIdentity, McpHost, PendingCall, ResourceChange, ResourceDescriptor,
    Scope, ScopeLadder, ScopeOf, ToolDescriptor, RESOURCE_MIME_TYPE,
};
pub use http::{
    read_one_response, read_request, read_request_head, read_response, read_response_head,
    read_response_head_timeout, read_response_timeout, reason_phrase, write_response, EventStream,
    HeaderMap, HttpError, HttpLimits, HttpMethod, HttpRequest, HttpResponse, HttpResponseHead,
    SseEvent, SseOpenError, SseStream,
};
pub use jsonrpc::{
    encode_error, encode_notification, encode_result, parse_message, JsonRpcAnswer, JsonRpcError,
    JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, RequestId, INTERNAL_ERROR, INVALID_PARAMS,
    INVALID_REQUEST, JSON_RPC_VERSION, METHOD_NOT_FOUND, PARSE_ERROR,
};
pub use oauth::{
    parse_jwks, JsonWebKey, OAuthError, OAuthIssuer, OAuthResource, OAuthScopeNames,
    DEFAULT_LEEWAY_SECONDS, MAX_MODULUS_BITS, MIN_MODULUS_BITS,
};
pub use server::{
    check_bind, BindRefusal, McpServer, McpServerConfig, RemoteAuthority,
    CLIENT_OPERATION_URI_PREFIX, OPERATION_URI_PREFIX, PROTECTED_RESOURCE_METADATA_PATH,
    PROTOCOL_VERSION, PROTOCOL_VERSION_HEADER, RESOURCE_NOT_FOUND, SESSION_HEADER,
};
pub use session::{
    SessionError, SessionEvents, SessionLimits, SessionRegistry, SessionRequestGuard, SessionView,
    DEFAULT_SESSION_IDLE_TTL,
};
pub use tls::{pem_blocks, tls_acceptor, TlsError, TlsMaterial};

#[cfg(test)]
mod tests;
