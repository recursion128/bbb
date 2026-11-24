//! JSON-RPC 2.0 framing, pinned to the revision MCP declares.
//!
//! Vanilla owner: `net.minecraft.server.jsonrpc.JsonRPCUtils` and
//! `JsonRPCErrors` carry the same envelope and the same reserved error codes
//! over a WebSocket. The codes below are the JSON-RPC 2.0 specification's own,
//! so both servers refuse a malformed document the same way.
//!
//! One deliberate divergence: batched requests are refused. The MCP revision
//! this server pins removed JSON-RPC batching, so accepting an array would
//! answer in a shape no pinned client is allowed to send.

use serde_json::{json, Map, Value};

pub const JSON_RPC_VERSION: &str = "2.0";

pub const PARSE_ERROR: i32 = -32700;
pub const INVALID_REQUEST: i32 = -32600;
pub const METHOD_NOT_FOUND: i32 = -32601;
pub const INVALID_PARAMS: i32 = -32602;
pub const INTERNAL_ERROR: i32 = -32603;

/// A request identity. JSON-RPC allows a null id; MCP forbids it, so this type
/// cannot represent one.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RequestId {
    Number(i64),
    Text(String),
}

impl RequestId {
    pub fn to_value(&self) -> Value {
        match self {
            Self::Number(number) => Value::from(*number),
            Self::Text(text) => Value::from(text.clone()),
        }
    }

    pub fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::Number(number) => number.as_i64().map(Self::Number),
            Value::String(text) => Some(Self::Text(text.clone())),
            _ => None,
        }
    }

    /// The stable text form used for idempotency records and audit lines.
    pub fn as_text(&self) -> String {
        match self {
            Self::Number(number) => number.to_string(),
            Self::Text(text) => text.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

impl JsonRpcError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }

    pub fn parse_error(detail: impl Into<String>) -> Self {
        Self::new(PARSE_ERROR, detail)
    }

    pub fn invalid_request(detail: impl Into<String>) -> Self {
        Self::new(INVALID_REQUEST, detail)
    }

    pub fn method_not_found(method: &str) -> Self {
        Self::new(METHOD_NOT_FOUND, format!("unknown method {method}"))
    }

    pub fn invalid_params(detail: impl Into<String>) -> Self {
        Self::new(INVALID_PARAMS, detail)
    }

    pub fn internal(detail: impl Into<String>) -> Self {
        Self::new(INTERNAL_ERROR, detail)
    }

    pub fn to_value(&self) -> Value {
        let mut error = Map::new();
        error.insert("code".to_string(), Value::from(self.code));
        error.insert("message".to_string(), Value::from(self.message.clone()));
        if let Some(data) = &self.data {
            error.insert("data".to_string(), data.clone());
        }
        Value::Object(error)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct JsonRpcRequest {
    pub id: RequestId,
    pub method: String,
    pub params: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JsonRpcNotification {
    pub method: String,
    pub params: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JsonRpcAnswer {
    pub id: RequestId,
    pub outcome: Result<Value, JsonRpcError>,
}

/// One decoded JSON-RPC document. A client may send a response of its own when
/// it answers a server-initiated request, so that shape is decoded rather than
/// refused even though this server initiates none.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonRpcMessage {
    Request(JsonRpcRequest),
    Notification(JsonRpcNotification),
    Answer(JsonRpcAnswer),
}

impl JsonRpcMessage {
    /// Whether the sender is waiting for an answer, which is what decides
    /// between `202 Accepted` and a body.
    pub fn expects_answer(&self) -> bool {
        matches!(self, Self::Request(_))
    }
}

pub fn parse_message(bytes: &[u8]) -> Result<JsonRpcMessage, JsonRpcError> {
    let document: Value = serde_json::from_slice(bytes)
        .map_err(|error| JsonRpcError::parse_error(format!("malformed JSON: {error}")))?;
    if document.is_array() {
        return Err(JsonRpcError::invalid_request(
            "batched requests are not part of this protocol revision",
        ));
    }
    let Value::Object(document) = document else {
        return Err(JsonRpcError::invalid_request(
            "a JSON-RPC message is an object",
        ));
    };
    match document.get("jsonrpc").and_then(Value::as_str) {
        Some(JSON_RPC_VERSION) => {}
        Some(other) => {
            return Err(JsonRpcError::invalid_request(format!(
                "jsonrpc {other} is not {JSON_RPC_VERSION}"
            )))
        }
        None => {
            return Err(JsonRpcError::invalid_request(
                "a JSON-RPC message carries a jsonrpc version",
            ))
        }
    }

    if let Some(method) = document.get("method") {
        let Some(method) = method.as_str() else {
            return Err(JsonRpcError::invalid_request("a method name is a string"));
        };
        let params = match document.get("params") {
            None | Some(Value::Null) => Value::Object(Map::new()),
            Some(Value::Object(params)) => Value::Object(params.clone()),
            Some(_) => {
                return Err(JsonRpcError::invalid_params(
                    "params are named, not positional",
                ))
            }
        };
        return match document.get("id") {
            None => Ok(JsonRpcMessage::Notification(JsonRpcNotification {
                method: method.to_string(),
                params,
            })),
            Some(id) => {
                let id = RequestId::from_value(id).ok_or_else(|| {
                    JsonRpcError::invalid_request("a request id is a string or a number")
                })?;
                Ok(JsonRpcMessage::Request(JsonRpcRequest {
                    id,
                    method: method.to_string(),
                    params,
                }))
            }
        };
    }

    let Some(id) = document.get("id").and_then(RequestId::from_value) else {
        return Err(JsonRpcError::invalid_request(
            "a JSON-RPC message carries a method or an answered id",
        ));
    };
    if let Some(error) = document.get("error") {
        let code = error
            .get("code")
            .and_then(Value::as_i64)
            .ok_or_else(|| JsonRpcError::invalid_request("an error carries a numeric code"))?;
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        return Ok(JsonRpcMessage::Answer(JsonRpcAnswer {
            id,
            outcome: Err(JsonRpcError {
                code: code as i32,
                message,
                data: error.get("data").cloned(),
            }),
        }));
    }
    let Some(result) = document.get("result") else {
        return Err(JsonRpcError::invalid_request(
            "an answer carries a result or an error",
        ));
    };
    Ok(JsonRpcMessage::Answer(JsonRpcAnswer {
        id,
        outcome: Ok(result.clone()),
    }))
}

pub fn encode_result(id: &RequestId, result: Value) -> Value {
    json!({ "jsonrpc": JSON_RPC_VERSION, "id": id.to_value(), "result": result })
}

/// Encodes a failure. An id is absent only when the document was too malformed
/// to carry one, which JSON-RPC answers with a null id.
pub fn encode_error(id: Option<&RequestId>, error: &JsonRpcError) -> Value {
    json!({
        "jsonrpc": JSON_RPC_VERSION,
        "id": id.map_or(Value::Null, RequestId::to_value),
        "error": error.to_value(),
    })
}

pub fn encode_notification(method: &str, params: Value) -> Value {
    json!({ "jsonrpc": JSON_RPC_VERSION, "method": method, "params": params })
}
