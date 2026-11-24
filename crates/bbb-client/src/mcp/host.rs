//! The client as an MCP host over its control state.
//!
//! Every tool call is the control request of the same name handed to
//! the control dispatcher, so the two protocols cannot drift: what a
//! harness line asks for and what an agent's tool call asks for reach the
//! same function with the same parameters. A control refusal (a missing
//! parameter, a closed command owner) is an execution failure and travels as
//! `isError` content; only a name the catalog does not know is a protocol
//! error.

use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use bbb_client_clock::UNIX_EPOCH;
use bbb_mcp::{
    control::{
        catalog, client_host_identity, encode_client_outcome_record,
        encode_client_resource_contents, encode_client_tool_result, resources, tool_by_name,
        tool_by_resource_uri, ClientMcpOutcome, ClientTool, ControlResponse as McpControlResponse,
        ControlScreenshot,
    },
    CallRefusal, Caller, HostIdentity, JsonRpcError, McpHost, PendingCall, ResourceChange,
    ResourceDescriptor, Scope, ToolDescriptor,
};
use serde_json::{json, Value};
use tokio::sync::broadcast;

use crate::control::{
    dispatch, dispatch_confirmed, ControlCompletion, ControlRequest, ControlResponse, ControlState,
};

/// How many change notifications a slow fan-out may fall behind before it
/// re-announces every resource.
const CHANGE_CHANNEL_CAPACITY: usize = 256;

/// One answered control request.
pub type ClientOutcome = ClientMcpOutcome;

#[derive(Clone)]
pub struct ClientHost {
    control: ControlState,
    next_operation: Arc<AtomicU64>,
    changes: broadcast::Sender<ResourceChange>,
}

impl ClientHost {
    pub fn new(control: ControlState) -> Self {
        Self {
            control,
            next_operation: Arc::new(AtomicU64::new(1)),
            changes: broadcast::channel(CHANGE_CHANNEL_CAPACITY).0,
        }
    }

    fn current_generation(&self) -> u64 {
        self.control.snapshot.load().generation
    }

    /// Tells subscribers every resource may have moved.
    pub fn publish_changes(&self) {
        for resource in resources() {
            let _ = self.changes.send(ResourceChange { uri: resource.uri });
        }
    }

    /// Polls the snapshot generation and publishes a change whenever it moved.
    /// The control state has no wake-up of its own, so this is the one place
    /// a subscription learns that the world advanced.
    pub fn spawn_change_watch(&self, interval: Duration) -> tokio::task::JoinHandle<()> {
        let host = self.clone();
        // Read before the task runs, so a bump between spawn and first poll is
        // not mistaken for the starting point.
        let mut seen = host.current_generation();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;
                let generation = host.current_generation();
                if generation != seen {
                    seen = generation;
                    host.publish_changes();
                }
            }
        })
    }

    fn answer(&self, tool: &'static ClientTool, params: Value) -> PendingCall<ClientOutcome> {
        let operation_id = self.next_operation.fetch_add(1, Ordering::Relaxed);
        let control = &self.control;
        let request = ControlRequest {
            method: tool.name.to_string(),
            params,
        };
        if tool.read_only {
            if crate::control::method_reads_snapshot(tool.name) {
                control.publication.record_demand();
                if control.publication.needs_flush() {
                    let control = control.clone();
                    let action = tool.name;
                    let (sender, answer) = tokio::sync::oneshot::channel();
                    tokio::spawn(async move {
                        control.ensure_fresh_snapshot().await;
                        let response = dispatch(request, &control);
                        let _ = sender.send(ClientOutcome {
                            operation_id,
                            action,
                            response: to_mcp_response(response),
                            image: None,
                        });
                    });
                    return PendingCall {
                        operation_id,
                        action: tool.name,
                        answer,
                    };
                }
            }
            let outcome = ClientOutcome {
                operation_id,
                action: tool.name,
                response: to_mcp_response(dispatch(request, control)),
                image: None,
            };
            return PendingCall::answered(operation_id, tool.name, outcome);
        }
        control.publication.record_demand();
        let (response, completion) = dispatch_confirmed(request, control);
        let outcome = ClientOutcome {
            operation_id,
            action: tool.name,
            response: to_mcp_response(response),
            image: None,
        };
        let Some(completion) = completion.filter(|_| outcome.response.ok) else {
            return PendingCall::answered(operation_id, tool.name, outcome);
        };
        let (sender, answer) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let mut outcome = outcome;
            match completion
                .await
                .unwrap_or_else(|_| Err("client stopped before completing the request".into()))
            {
                Ok(ControlCompletion::Recording(status)) => {
                    outcome.response.result = Some(json!({"completed": true, "recording": status}));
                }
                Ok(ControlCompletion::Processed { generation }) => {
                    if let Some(Value::Object(result)) = outcome.response.result.as_mut() {
                        result.remove("queued");
                        result.remove("pending");
                        result.insert("processed".into(), json!(true));
                        result.insert("generation".into(), json!(generation));
                    }
                }
                Ok(ControlCompletion::Screenshot { path, image }) => {
                    outcome.response.result = Some(json!({
                        "completed": true, "path": path,
                        "width": image.width, "height": image.height, "mime_type": image.mime_type,
                    }));
                    outcome.image = Some(ControlScreenshot {
                        width: image.width,
                        height: image.height,
                        mime_type: image.mime_type,
                        bytes: image.bytes,
                    });
                }
                Err(error) => {
                    outcome.response = McpControlResponse::err(error);
                }
            }
            let _ = sender.send(outcome);
        });
        PendingCall {
            operation_id,
            action: tool.name,
            answer,
        }
    }
}

fn to_mcp_response(response: ControlResponse) -> McpControlResponse {
    McpControlResponse {
        ok: response.ok,
        result: response.result,
        error: response.error,
    }
}

impl std::fmt::Debug for ClientHost {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("ClientHost").finish_non_exhaustive()
    }
}

impl McpHost for ClientHost {
    type Scopes = Scope;
    type Outcome = ClientOutcome;

    fn identity() -> HostIdentity {
        client_host_identity()
    }

    fn now(&self) -> i64 {
        bbb_client_clock::system_now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX))
            .unwrap_or(0)
    }

    fn tools(&self) -> Vec<ToolDescriptor> {
        catalog().iter().map(ClientTool::descriptor).collect()
    }

    fn resources(&self) -> Vec<ResourceDescriptor> {
        resources()
    }

    fn resource(&self, uri: &str) -> Option<ResourceDescriptor> {
        tool_by_resource_uri(uri).and_then(ClientTool::resource)
    }

    fn call_tool(
        &self,
        caller: &Caller<Scope>,
        name: &str,
        arguments: &Value,
    ) -> Result<PendingCall<ClientOutcome>, CallRefusal> {
        let tool = tool_by_name(name).ok_or(CallRefusal::Unknown)?;
        let required = if tool.read_only {
            Scope::Read
        } else {
            Scope::Manage
        };
        if caller.scope < required {
            let operation_id = self.next_operation.fetch_add(1, Ordering::Relaxed);
            let outcome = ClientOutcome {
                operation_id,
                action: tool.name,
                response: McpControlResponse::err(format!(
                    "scope `{}` is insufficient for `{}`; requires `{}`",
                    caller.scope.name(),
                    tool.name,
                    required.name()
                )),
                image: None,
            };
            return Ok(PendingCall::answered(operation_id, tool.name, outcome));
        }
        Ok(self.answer(tool, arguments.clone()))
    }

    fn read_resource(
        &self,
        _caller: &Caller<Scope>,
        uri: &str,
    ) -> Result<PendingCall<ClientOutcome>, CallRefusal> {
        let tool = tool_by_resource_uri(uri).ok_or(CallRefusal::Unknown)?;
        Ok(self.answer(tool, Value::Null))
    }

    fn tool_result(&self, outcome: &ClientOutcome) -> Value {
        encode_client_tool_result(outcome)
    }

    fn outcome_record(&self, outcome: &ClientOutcome) -> Value {
        encode_client_outcome_record(outcome)
    }

    fn resource_contents(&self, outcome: &ClientOutcome) -> Result<Value, JsonRpcError> {
        encode_client_resource_contents(outcome)
    }

    fn changes(&self) -> broadcast::Receiver<ResourceChange> {
        self.changes.subscribe()
    }
}
