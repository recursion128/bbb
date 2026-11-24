//! The MCP management adapter for the dedicated server.
//!
//! Vanilla 26.1 reaches the same administration semantics over a WebSocket
//! JSON-RPC management server (`net.minecraft.server.jsonrpc`); bbb speaks MCP
//! Streamable HTTP instead, through the transport `bbb_mcp` shares with the
//! client. This crate owns only the server's projection: the tool/resource
//! schema, the answer encoding, and the host adapter that submits every call
//! through the bounded world mailbox rather than by borrowing world state. The
//! semantics themselves live in `bbb_server_core::admin`.

mod encode;
mod gateway;
mod host;
mod resources;
mod tools;

// Kept strictly for `bbb-server` until Stage 4 switches `bbb-server` imports onto `bbb_mcp`.
pub use bbb_mcp::{
    check_bind, parse_jwks, tls_acceptor, OAuthIssuer, OAuthResource, OAuthScopeNames,
    RemoteAuthority, TlsMaterial, DEFAULT_LEEWAY_SECONDS,
};
pub use encode::{
    component_value, diagnostics_value, game_rule_type, game_rule_value, ip_ban_value,
    operator_value, player_value, result_text, result_value, server_state_value, setting_entry,
    setting_value, typed_game_rule_value, user_ban_value,
};
pub use gateway::{AdminCall, AdminGateway, GatewayError, PendingOperation, ResourceChange};
pub use host::{admin_result_content, AdminHost, AdminScopes, OPERATION_URI_PREFIX};
pub use resources::{
    resource_by_uri, resources, resources_changed_by, resources_changed_by_player_list,
    AdminResource,
};
pub use tools::{catalog, setting_from_json, tool_by_name, AdminTool};

/// The shared transport's types, fixed to the server's scope ladder and host.
pub type AuthConfig = bbb_mcp::AuthConfig<AdminScopes>;
pub type TokenGrant = bbb_mcp::TokenGrant<AdminScopes>;
pub type Principal = bbb_mcp::Principal<AdminScopes>;
pub type SessionRegistry = bbb_mcp::SessionRegistry<AdminScopes>;
pub type SessionView = bbb_mcp::SessionView<AdminScopes>;
pub type McpServerConfig = bbb_mcp::McpServerConfig<AdminHost>;
pub type McpServer = bbb_mcp::McpServer<AdminHost>;

#[cfg(test)]
mod tests;
