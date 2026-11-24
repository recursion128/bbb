use std::{
    collections::BTreeSet,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::control::{
    dispatch, ControlRequest, ControlRequestSink, ControlState, ControlSubmitError,
    ControlSubmitReceipt, ControlWriteRequest, NetControlRequest, SharedControlRequests,
};
use bbb_mcp::{McpHost, PROTOCOL_VERSION, SESSION_HEADER};
use serde_json::{json, Value};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::timeout,
};

use crate::mcp::{catalog, check_bind, resources, ClientHost, McpServer, McpServerConfig};

#[derive(Debug, Default)]
struct RecordingSink {
    requests: Mutex<Vec<ControlWriteRequest>>,
}

impl ControlRequestSink for RecordingSink {
    fn submit(
        &self,
        request: ControlWriteRequest,
    ) -> Result<ControlSubmitReceipt, ControlSubmitError> {
        let mut requests = self.requests.lock().expect("requests");
        requests.push(request);
        Ok(ControlSubmitReceipt {
            pending: requests.len(),
        })
    }

    fn submit_confirmed(
        &self,
        request: ControlWriteRequest,
        completion: crate::control::ControlCompletionSender,
    ) -> Result<ControlSubmitReceipt, ControlSubmitError> {
        let receipt = self.submit(request)?;
        let _ = completion.send(Ok(crate::control::ControlCompletion::Processed {
            generation: 1,
        }));
        Ok(receipt)
    }
}

fn control_state() -> (ControlState, Arc<RecordingSink>) {
    let sink = Arc::new(RecordingSink::default());
    let control = ControlState::new("test-client", SharedControlRequests::new(sink.clone()));
    (control, sink)
}

#[derive(Default)]
struct DeferredSink(Mutex<Vec<(ControlWriteRequest, crate::control::ControlCompletionSender)>>);

impl ControlRequestSink for DeferredSink {
    fn submit(
        &self,
        _request: ControlWriteRequest,
    ) -> Result<ControlSubmitReceipt, ControlSubmitError> {
        panic!("MCP writes must request completion");
    }

    fn submit_confirmed(
        &self,
        request: ControlWriteRequest,
        completion: crate::control::ControlCompletionSender,
    ) -> Result<ControlSubmitReceipt, ControlSubmitError> {
        self.0.lock().unwrap().push((request, completion));
        Ok(ControlSubmitReceipt { pending: 1 })
    }
}

#[tokio::test]
async fn writes_wait_for_processing_and_stopping_is_an_execution_failure() {
    let sink = Arc::new(DeferredSink::default());
    let control = ControlState::new("test", SharedControlRequests::new(sink.clone()));
    let host = ClientHost::new(control);
    let caller = bbb_mcp::Caller {
        principal: "test".into(),
        scope: bbb_mcp::Scope::Manage,
        request_id: "input".into(),
    };
    let mut pending = host
        .call_tool(&caller, "input.focus", &json!({"focused": true}))
        .unwrap();
    assert!(matches!(
        pending.answer.try_recv(),
        Err(tokio::sync::oneshot::error::TryRecvError::Empty)
    ));
    let (_, completion) = sink.0.lock().unwrap().pop().unwrap();
    completion
        .send(Ok(crate::control::ControlCompletion::Processed {
            generation: 17,
        }))
        .unwrap();
    let outcome = pending.answer.await.unwrap();
    let result = host.tool_result(&outcome);
    assert_eq!(result["structuredContent"]["processed"], true);
    assert_eq!(result["structuredContent"]["generation"], 17);
    assert!(result["structuredContent"].get("queued").is_none());

    let pending = host
        .call_tool(
            &caller,
            "renderer.screenshot",
            &json!({"path": "never-written.png"}),
        )
        .unwrap();
    sink.0.lock().unwrap().clear();
    let outcome = pending.answer.await.unwrap();
    assert_eq!(host.tool_result(&outcome)["isError"], true);
}

fn loopback() -> SocketAddr {
    "127.0.0.1:52000".parse().expect("address")
}

struct RawResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

impl RawResponse {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header, _)| header.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    fn json(&self) -> Value {
        serde_json::from_str(&self.body)
            .unwrap_or_else(|error| panic!("body is JSON: {error}; body was {:?}", self.body))
    }

    fn result(&self) -> Value {
        let body = self.json();
        assert!(
            body.get("error").is_none(),
            "expected a result, got {}",
            self.body
        );
        body["result"].clone()
    }

    fn error_code(&self) -> i64 {
        self.json()["error"]["code"]
            .as_i64()
            .unwrap_or_else(|| panic!("expected an error, got {}", self.body))
    }
}

async fn exchange(server: &Arc<McpServer>, raw: String) -> RawResponse {
    let (mut client, service) = tokio::io::duplex(256 * 1024);
    let handler = tokio::spawn(Arc::clone(server).handle_connection(service, loopback()));
    client.write_all(raw.as_bytes()).await.expect("write");
    let mut received = Vec::new();
    timeout(Duration::from_secs(5), client.read_to_end(&mut received))
        .await
        .expect("the answer arrives")
        .expect("read");
    handler.await.expect("the connection task finishes");
    let text = String::from_utf8_lossy(&received).to_string();
    let (head, body) = text.split_once("\r\n\r\n").expect("a response head");
    let mut lines = head.split("\r\n");
    let status = lines
        .next()
        .and_then(|line| line.split(' ').nth(1))
        .and_then(|status| status.parse::<u16>().ok())
        .expect("a status code");
    let headers = lines
        .filter_map(|line| line.split_once(": "))
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect();
    RawResponse {
        status,
        headers,
        body: body.to_string(),
    }
}

fn post(body: &Value, session: Option<&str>) -> String {
    let body = body.to_string();
    let mut request = String::from("POST /mcp HTTP/1.1\r\nhost: 127.0.0.1\r\n");
    request.push_str("accept: application/json, text/event-stream\r\n");
    request.push_str("content-type: application/json\r\n");
    request.push_str("connection: close\r\n");
    if let Some(session) = session {
        request.push_str(&format!("{SESSION_HEADER}: {session}\r\n"));
    }
    request.push_str(&format!("content-length: {}\r\n\r\n", body.len()));
    request.push_str(&body);
    request
}

fn call(id: i64, method: &str, params: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
}

async fn started(control: ControlState) -> (Arc<McpServer>, String) {
    let server = McpServer::new(McpServerConfig::default(), ClientHost::new(control));
    let response = exchange(
        &server,
        post(
            &call(
                1,
                "initialize",
                json!({ "protocolVersion": PROTOCOL_VERSION, "capabilities": {} }),
            ),
            None,
        ),
    )
    .await;
    assert_eq!(response.status, 200, "{}", response.body);
    let result = response.result();
    assert_eq!(result["serverInfo"]["name"], "bbb-client");
    assert_eq!(result["_meta"]["bbb/scope"], "manage");
    let session = response
        .header(SESSION_HEADER)
        .expect("a session id")
        .to_string();
    let ready = exchange(
        &server,
        post(
            &json!({ "jsonrpc": "2.0", "method": "notifications/initialized", "params": {} }),
            Some(&session),
        ),
    )
    .await;
    assert_eq!(ready.status, 202, "{}", ready.body);
    (server, session)
}

#[test]
fn the_catalog_is_exactly_the_control_dispatch() {
    let listed = catalog()
        .iter()
        .map(|tool| tool.name.to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(listed.len(), catalog().len(), "no tool is listed twice");

    // A readout takes no argument exactly when it is also a resource, and every
    // resource uri is the method's own name under the client prefix.
    for tool in catalog() {
        let schema = (tool.schema)();
        let has_parameters = schema["properties"]
            .as_object()
            .is_some_and(|properties| !properties.is_empty());
        match tool.resource_uri {
            Some(uri) => {
                assert!(tool.read_only, "{}", tool.name);
                assert!(!has_parameters, "{}", tool.name);
                assert_eq!(uri, format!("bbb://client/{}", tool.name.replace('.', "/")));
            }
            None => assert!(!tool.read_only || has_parameters, "{}", tool.name),
        }
        let descriptor = tool.descriptor().to_value();
        assert_eq!(descriptor["annotations"]["readOnlyHint"], tool.read_only);
        assert_eq!(descriptor["_meta"]["bbb/controlMethod"], tool.name);
    }
    assert_eq!(
        resources().len(),
        catalog()
            .iter()
            .filter(|tool| tool.resource_uri.is_some())
            .count()
    );

    // No tool name is refused by the dispatcher as unknown.
    let (control, _sink) = control_state();
    for tool in catalog() {
        let response = dispatch(
            ControlRequest {
                method: tool.name.to_string(),
                params: json!({}),
            },
            &control,
        );
        assert!(
            !response
                .error
                .as_deref()
                .is_some_and(|error| error.starts_with("unknown method")),
            "{}: {:?}",
            tool.name,
            response.error
        );
    }
}

#[tokio::test]
async fn tools_list_is_the_catalog_and_resources_list_is_its_readouts() {
    let (control, _sink) = control_state();
    let (server, session) = started(control).await;
    let tools = exchange(
        &server,
        post(&call(2, "tools/list", json!({})), Some(&session)),
    )
    .await
    .result();
    let tools = tools["tools"].as_array().expect("tools");
    assert_eq!(tools.len(), catalog().len());
    let chat = tools
        .iter()
        .find(|tool| tool["name"] == "net.send_chat_message")
        .expect("the chat tool is listed");
    assert_eq!(chat["inputSchema"]["required"][0], "message");
    assert_eq!(chat["annotations"]["readOnlyHint"], false);
    let status = tools
        .iter()
        .find(|tool| tool["name"] == "app.status")
        .expect("the status tool is listed");
    assert_eq!(status["annotations"]["readOnlyHint"], true);

    let listed = exchange(
        &server,
        post(&call(3, "resources/list", json!({})), Some(&session)),
    )
    .await
    .result();
    assert_eq!(
        listed["resources"].as_array().expect("resources").len(),
        resources().len()
    );
    assert!(listed["resources"]
        .as_array()
        .expect("resources")
        .iter()
        .any(|resource| resource["uri"] == "bbb://client/world/client_inventory"));
}

#[tokio::test]
async fn a_readout_tool_call_answers_the_control_snapshot() {
    let (control, _sink) = control_state();
    let (server, session) = started(control).await;
    let response = exchange(
        &server,
        post(
            &call(
                2,
                "tools/call",
                json!({ "name": "app.status", "arguments": {} }),
            ),
            Some(&session),
        ),
    )
    .await;
    let result = response.result();
    assert_eq!(result["isError"], false);
    assert_eq!(result["structuredContent"]["app"]["version"], "test-client");
    assert_eq!(result["structuredContent"]["app"]["running"], true);
    assert_eq!(result["_meta"]["bbb/operationId"], 1);
    assert!(result["content"][0]["text"]
        .as_str()
        .expect("text")
        .contains("test-client"));

    let read = exchange(
        &server,
        post(
            &call(
                3,
                "resources/read",
                json!({ "uri": "bbb://client/app/status" }),
            ),
            Some(&session),
        ),
    )
    .await
    .result();
    assert_eq!(read["contents"][0]["uri"], "bbb://client/app/status");
    let contents: Value = serde_json::from_str(read["contents"][0]["text"].as_str().expect("text"))
        .expect("the resource is JSON");
    assert_eq!(contents["app"]["version"], "test-client");

    let probe = exchange(
        &server,
        post(
            &call(
                4,
                "tools/call",
                json!({ "name": "world.probe_block", "arguments": { "x": 0, "y": 64, "z": 0 } }),
            ),
            Some(&session),
        ),
    )
    .await
    .result();
    assert_eq!(probe["isError"], false);
    assert_eq!(probe["structuredContent"], Value::Null);
}

#[tokio::test]
async fn a_write_tool_call_reaches_the_control_sink_and_a_refusal_is_content() {
    let (control, sink) = control_state();
    let (server, session) = started(control).await;
    let response = exchange(
        &server,
        post(
            &call(
                2,
                "tools/call",
                json!({ "name": "net.send_chat_message", "arguments": { "message": "hello" } }),
            ),
            Some(&session),
        ),
    )
    .await;
    let result = response.result();
    assert_eq!(result["isError"], false);
    assert_eq!(result["structuredContent"]["processed"], true);
    assert_eq!(result["structuredContent"]["generation"], 1);
    assert!(result["structuredContent"].get("pending").is_none());
    assert_eq!(
        sink.requests.lock().expect("requests").as_slice(),
        &[ControlWriteRequest::Net(NetControlRequest::ChatMessage {
            message: "hello".to_string(),
        })]
    );

    let refused = exchange(
        &server,
        post(
            &call(
                3,
                "tools/call",
                json!({ "name": "net.send_chat_message", "arguments": { "message": "" } }),
            ),
            Some(&session),
        ),
    )
    .await
    .result();
    assert_eq!(refused["isError"], true);
    assert!(refused["content"][0]["text"]
        .as_str()
        .expect("text")
        .contains("requires non-empty string param message"));
    assert_eq!(sink.requests.lock().expect("requests").len(), 1);

    let unknown = exchange(
        &server,
        post(
            &call(4, "tools/call", json!({ "name": "net.launch_rockets" })),
            Some(&session),
        ),
    )
    .await;
    assert_eq!(unknown.error_code(), i64::from(bbb_mcp::INVALID_PARAMS));

    let record = exchange(
        &server,
        post(
            &call(
                5,
                "resources/read",
                json!({ "uri": format!("{}2", crate::mcp::OPERATION_URI_PREFIX) }),
            ),
            Some(&session),
        ),
    )
    .await
    .result();
    let record: Value = serde_json::from_str(record["contents"][0]["text"].as_str().expect("text"))
        .expect("the record is JSON");
    assert_eq!(record["state"], "applied");
    assert_eq!(record["action"], "net.send_chat_message");
    assert_eq!(record["outcome"]["ok"], false);
}

#[tokio::test]
async fn a_generation_bump_is_published_to_every_resource_subscriber() {
    let (control, _sink) = control_state();
    let host = ClientHost::new(control.clone());
    let mut changes = host.changes();
    let watch = host.spawn_change_watch(Duration::from_millis(5));
    control.snapshot.rcu(|current| {
        let mut next = (**current).clone();
        next.generation += 1;
        next
    });
    let change = timeout(Duration::from_secs(5), changes.recv())
        .await
        .expect("a change is published")
        .expect("the channel is open");
    assert!(change.uri.starts_with("bbb://client/"));
    watch.abort();
}

#[test]
fn only_a_loopback_address_may_be_bound() {
    assert!(check_bind("127.0.0.1:25590".parse().expect("address")).is_ok());
    assert!(check_bind("0.0.0.0:25590".parse().expect("address")).is_err());
}

#[tokio::test]
async fn read_scope_caller_is_refused_write_tools_but_allowed_reads() {
    let (control, _sink) = control_state();
    let host = ClientHost::new(control);
    let read_caller = bbb_mcp::Caller {
        principal: "reader".to_string(),
        scope: bbb_mcp::Scope::Read,
        request_id: "req-1".to_string(),
    };
    let write_call = host
        .call_tool(&read_caller, "app.quit", &serde_json::json!({}))
        .expect("refusal travels as outcome");
    let write_outcome = write_call.answer.await.expect("answered");
    assert!(!write_outcome.response.ok);
    assert!(write_outcome
        .response
        .error
        .as_deref()
        .is_some_and(|msg| msg.contains("insufficient")));

    let read_call = host
        .call_tool(&read_caller, "app.status", &serde_json::json!({}))
        .expect("read allowed");
    let read_outcome = read_call.answer.await.expect("answered");
    assert!(read_outcome.response.ok);
}
