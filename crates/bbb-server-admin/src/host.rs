//! The dedicated server as an MCP host.
//!
//! Everything the shared transport needs from the server is answered here:
//! the tool catalog and resource table are projected from `tools.rs` and
//! `resources.rs`, every call becomes an [`AdminCall`] submitted through the
//! [`AdminGateway`], and an [`AdminOutcome`] is encoded into the tool result,
//! the operation record and the resource contents the protocol expects.

use std::sync::Arc;

use bbb_mcp::{
    CallRefusal, Caller, HostIdentity, JsonRpcError, McpHost, PendingCall, ResourceChange,
    ResourceDescriptor, ScopeLadder, ToolDescriptor, INTERNAL_ERROR,
};
use bbb_server_core::admin::{AdminOutcome, AdminRequest, AdminResult, AdminScope};
use serde_json::{json, Map, Value};
use tokio::sync::broadcast;

use crate::{
    encode::{result_text, result_value},
    gateway::{AdminCall, AdminGateway, GatewayError},
    resources::{resource_by_uri, resources},
    tools::{catalog, tool_by_name},
};

/// The uri prefix under which one submitted operation's own record is readable.
/// A call that outlived its deadline is still applied by the tick, so its answer
/// has to remain reachable by id rather than being lost with the request.
pub const OPERATION_URI_PREFIX: &str = "bbb://server/operation/";

/// The world's own scope ladder, as the transport authenticates against it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AdminScopes;

impl ScopeLadder for AdminScopes {
    type Scope = AdminScope;
    const READ: AdminScope = AdminScope::Read;
    const MANAGE: AdminScope = AdminScope::Manage;
    const COMMAND: AdminScope = AdminScope::Command;

    fn name(scope: AdminScope) -> &'static str {
        scope.name()
    }
}

/// The server's host adapter: a gateway, seen through the transport's seam.
#[derive(Clone)]
pub struct AdminHost {
    gateway: Arc<dyn AdminGateway>,
}

impl AdminHost {
    pub fn new(gateway: Arc<dyn AdminGateway>) -> Self {
        Self { gateway }
    }

    pub fn gateway(&self) -> &Arc<dyn AdminGateway> {
        &self.gateway
    }

    fn submit(
        &self,
        caller: &Caller<AdminScope>,
        request: AdminRequest,
    ) -> Result<PendingCall<AdminOutcome>, CallRefusal> {
        let action = request.action();
        let pending = self
            .gateway
            .submit(AdminCall {
                principal: caller.principal.clone(),
                scope: caller.scope,
                request_id: caller.request_id.clone(),
                request,
            })
            .map_err(|error| CallRefusal::Unavailable {
                detail: error.to_string(),
                retryable: error == GatewayError::Busy,
            })?;
        Ok(PendingCall {
            operation_id: pending.operation_id,
            action,
            answer: pending.answer,
        })
    }
}

impl From<Arc<dyn AdminGateway>> for AdminHost {
    fn from(gateway: Arc<dyn AdminGateway>) -> Self {
        Self::new(gateway)
    }
}

impl<G: AdminGateway> From<Arc<G>> for AdminHost {
    fn from(gateway: Arc<G>) -> Self {
        Self::new(gateway)
    }
}

impl std::fmt::Debug for AdminHost {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("AdminHost").finish_non_exhaustive()
    }
}

impl McpHost for AdminHost {
    type Scopes = AdminScopes;
    type Outcome = AdminOutcome;

    fn identity() -> HostIdentity {
        HostIdentity {
            name: "bbb-server",
            title: "BBB dedicated server administration",
            version: env!("CARGO_PKG_VERSION"),
            instructions: Some(
                "Administration for a bbb dedicated server. Read server state through the \
                 bbb://server/* resources and inspect loaded terrain through the world.* tools. \
                 Change state through the typed tools and reach for command.execute only when \
                 no typed tool covers what you need.",
            ),
            operation_uri_prefix: OPERATION_URI_PREFIX,
        }
    }

    fn now(&self) -> i64 {
        self.gateway.now()
    }

    fn tools(&self) -> Vec<ToolDescriptor> {
        catalog()
            .iter()
            .map(|tool| tool.descriptor_entry())
            .collect()
    }

    fn resources(&self) -> Vec<ResourceDescriptor> {
        resources()
            .iter()
            .map(|resource| resource.descriptor_entry())
            .collect()
    }

    fn resource(&self, uri: &str) -> Option<ResourceDescriptor> {
        resource_by_uri(uri).map(|resource| resource.descriptor_entry())
    }

    fn call_tool(
        &self,
        caller: &Caller<AdminScope>,
        name: &str,
        arguments: &Value,
    ) -> Result<PendingCall<AdminOutcome>, CallRefusal> {
        let tool = tool_by_name(name).ok_or(CallRefusal::Unknown)?;
        let request = (tool.decode)(arguments)
            .map_err(|error| CallRefusal::InvalidArguments(error.to_string()))?;
        self.submit(caller, request)
    }

    fn read_resource(
        &self,
        caller: &Caller<AdminScope>,
        uri: &str,
    ) -> Result<PendingCall<AdminOutcome>, CallRefusal> {
        let resource = resource_by_uri(uri).ok_or(CallRefusal::Unknown)?;
        self.submit(caller, (resource.request)())
    }

    fn tool_result(&self, outcome: &AdminOutcome) -> Value {
        tool_result(outcome)
    }

    fn outcome_record(&self, outcome: &AdminOutcome) -> Value {
        outcome_value(outcome)
    }

    fn resource_contents(&self, outcome: &AdminOutcome) -> Result<Value, JsonRpcError> {
        match &outcome.result {
            Ok(result) => Ok(result_value(result)),
            Err(error) => Err(JsonRpcError::new(INTERNAL_ERROR, error.to_string())
                .with_data(json!({ "code": error.code() }))),
        }
    }

    fn changes(&self) -> broadcast::Receiver<ResourceChange> {
        self.gateway.subscribe()
    }
}

fn outcome_value(outcome: &AdminOutcome) -> Value {
    let mut object = Map::new();
    object.insert("operationId".to_string(), Value::from(outcome.operation_id));
    object.insert(
        "requestId".to_string(),
        Value::from(outcome.request_id.clone()),
    );
    object.insert(
        "principal".to_string(),
        Value::from(outcome.principal.clone()),
    );
    object.insert("scope".to_string(), Value::from(outcome.scope.name()));
    object.insert("action".to_string(), Value::from(outcome.action));
    object.insert("tick".to_string(), Value::from(outcome.tick));
    object.insert("replayed".to_string(), Value::from(outcome.replayed));
    match &outcome.result {
        Ok(result) => {
            object.insert("result".to_string(), result_value(result));
        }
        Err(error) => {
            object.insert(
                "error".to_string(),
                json!({ "code": error.code(), "detail": error.to_string() }),
            );
        }
    }
    Value::Object(object)
}

/// One applied request as an MCP tool result.
///
/// A refusal the world decided — a scope it does not satisfy, a deadline it
/// missed, an argument it will not take — is an execution failure, so it
/// travels as `isError` content rather than as a protocol error. Confusing the
/// two would tell an agent its request was malformed when it was refused.
fn tool_result(outcome: &AdminOutcome) -> Value {
    let meta = json!({
        "bbb/operationId": outcome.operation_id,
        "bbb/tick": outcome.tick,
        "bbb/replayed": outcome.replayed,
    });
    match &outcome.result {
        Ok(result) => json!({
            "content": [{ "type": "text", "text": result_text(result) }],
            "structuredContent": result_value(result),
            "isError": false,
            "_meta": meta,
        }),
        Err(error) => json!({
            "content": [{
                "type": "text",
                "text": format!("{} refused: {error}", outcome.action),
            }],
            "isError": true,
            "_meta": meta,
        }),
    }
}

/// Re-exported so a consumer can answer the same shape a tool call answers.
pub fn admin_result_content(result: &AdminResult) -> Value {
    json!({
        "content": [{ "type": "text", "text": result_text(result) }],
        "structuredContent": result_value(result),
        "isError": false,
    })
}
