//! The seam between the shared MCP transport and whatever it manages.
//!
//! The transport owns framing, sessions, authentication, deadlines and the
//! operation records; a host owns the tool catalog, the resources and the
//! semantics behind them. The dedicated server implements this over its
//! bounded world mailbox and the client over its control snapshot, and neither
//! learns anything about HTTP or JSON-RPC.

use std::fmt;

use serde_json::{json, Value};
use tokio::sync::{broadcast, oneshot};

use crate::jsonrpc::JsonRpcError;

/// The media type every resource answers with.
pub const RESOURCE_MIME_TYPE: &str = "application/json";

/// The three-rung authority ladder the transport authenticates against.
///
/// The rungs are named by the host's own scope type so a host that already
/// owns one (the server's `AdminScope`) keeps it; the ladder only has to say
/// which value is which rung and how each is spelled on the wire.
pub trait ScopeLadder: Copy + Eq + fmt::Debug + Send + Sync + 'static {
    type Scope: Copy + Eq + Ord + fmt::Debug + Send + Sync + 'static;
    const READ: Self::Scope;
    const MANAGE: Self::Scope;
    const COMMAND: Self::Scope;
    fn name(scope: Self::Scope) -> &'static str;
}

/// The scope value a ladder names.
pub type ScopeOf<L> = <L as ScopeLadder>::Scope;

/// The built-in ladder for a host that has no scope type of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Scope {
    Read,
    Manage,
    Command,
}

impl Scope {
    pub fn name(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Manage => "manage",
            Self::Command => "command",
        }
    }

    pub fn by_name(name: &str) -> Option<Self> {
        Some(match name {
            "read" => Self::Read,
            "manage" => Self::Manage,
            "command" => Self::Command,
            _ => return None,
        })
    }
}

impl ScopeLadder for Scope {
    type Scope = Scope;
    const READ: Scope = Scope::Read;
    const MANAGE: Scope = Scope::Manage;
    const COMMAND: Scope = Scope::Command;

    fn name(scope: Scope) -> &'static str {
        scope.name()
    }
}

/// What a host calls itself, and the defaults its configuration starts from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostIdentity {
    pub name: &'static str,
    pub title: &'static str,
    pub version: &'static str,
    pub instructions: Option<&'static str>,
    /// The uri prefix under which one submitted operation's record is readable.
    pub operation_uri_prefix: &'static str,
}

/// One `tools/list` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolDescriptor {
    pub name: &'static str,
    pub title: &'static str,
    pub summary: &'static str,
    pub input_schema: Value,
    pub read_only: bool,
    pub destructive: bool,
    /// Extra `_meta` the host attaches, omitted when `None`.
    pub meta: Option<Value>,
}

impl ToolDescriptor {
    pub fn to_value(&self) -> Value {
        let mut descriptor = json!({
            "name": self.name,
            "title": self.title,
            "description": self.summary,
            "inputSchema": self.input_schema,
            "annotations": {
                "title": self.title,
                "readOnlyHint": self.read_only,
                "destructiveHint": self.destructive,
                "idempotentHint": false,
                "openWorldHint": false,
            },
        });
        if let Some(meta) = &self.meta {
            descriptor["_meta"] = meta.clone();
        }
        descriptor
    }
}

/// One `resources/list` entry. The uri is static because a session subscribes
/// to it by identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceDescriptor {
    pub uri: &'static str,
    pub name: &'static str,
    pub title: &'static str,
    pub summary: &'static str,
}

impl ResourceDescriptor {
    pub fn to_value(&self) -> Value {
        json!({
            "uri": self.uri,
            "name": self.name,
            "title": self.title,
            "description": self.summary,
            "mimeType": RESOURCE_MIME_TYPE,
        })
    }
}

/// The authority one call travels with, as the transport resolved it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Caller<S> {
    pub principal: String,
    pub scope: S,
    /// The idempotency key: two calls from one principal sharing it are the
    /// same operation.
    pub request_id: String,
}

/// A submitted call: its host-assigned id, the audit action it became, and
/// the answer the host will produce. A dropped sender means the host abandoned
/// the call.
#[derive(Debug)]
pub struct PendingCall<O> {
    pub operation_id: u64,
    pub action: &'static str,
    pub answer: oneshot::Receiver<O>,
}

impl<O> PendingCall<O> {
    /// A call the host answered on the spot.
    pub fn answered(operation_id: u64, action: &'static str, outcome: O) -> Self {
        let (sender, answer) = oneshot::channel();
        let _ = sender.send(outcome);
        Self {
            operation_id,
            action,
            answer,
        }
    }
}

/// Why a host did not submit a call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallRefusal {
    /// No tool or resource of that name.
    Unknown,
    /// The arguments do not decode to the call the tool makes.
    InvalidArguments(String),
    /// The host cannot take the call right now.
    Unavailable { detail: String, retryable: bool },
}

/// A resource whose content may have changed. Published by the host after it
/// applied something, consumed by the subscription fan-out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceChange {
    pub uri: &'static str,
}

pub trait McpHost: Send + Sync + 'static {
    type Scopes: ScopeLadder;
    /// One completed call's answer; the operation log retains its `outcome_record` projection.
    type Outcome: Send + 'static;

    fn identity() -> HostIdentity;

    /// Epoch milliseconds; the transport owns no clock.
    fn now(&self) -> i64;

    fn tools(&self) -> Vec<ToolDescriptor>;

    fn resources(&self) -> Vec<ResourceDescriptor>;

    fn resource(&self, uri: &str) -> Option<ResourceDescriptor> {
        self.resources()
            .into_iter()
            .find(|resource| resource.uri == uri)
    }

    /// Decodes and submits one tool call. Never blocks.
    fn call_tool(
        &self,
        caller: &Caller<ScopeOf<Self::Scopes>>,
        name: &str,
        arguments: &Value,
    ) -> Result<PendingCall<Self::Outcome>, CallRefusal>;

    /// Submits the read behind one resource. Never blocks.
    fn read_resource(
        &self,
        caller: &Caller<ScopeOf<Self::Scopes>>,
        uri: &str,
    ) -> Result<PendingCall<Self::Outcome>, CallRefusal>;

    /// The `tools/call` result an outcome becomes.
    fn tool_result(&self, outcome: &Self::Outcome) -> Value;

    /// The outcome as its operation record reports it.
    fn outcome_record(&self, outcome: &Self::Outcome) -> Value;

    /// The resource value an outcome carries, or the error a read answers with.
    fn resource_contents(&self, outcome: &Self::Outcome) -> Result<Value, JsonRpcError>;

    /// The change stream the subscription fan-out reads.
    fn changes(&self) -> broadcast::Receiver<ResourceChange>;
}
