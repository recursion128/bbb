//! Shared wire envelopes and endpoint contract for remote client control and
//! `/control-ws` MCP bridging without renderer or image-crate dependencies.

mod catalog;
mod remote_host;

use std::{future::Future, pin::Pin};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde_json::{json, Map, Value};
use thiserror::Error;

pub use crate::server::{CLIENT_OPERATION_URI_PREFIX, OPERATION_URI_PREFIX};
pub use catalog::{
    catalog, resources, tool_by_name, tool_by_resource_uri, ClientTool, INPUT_KEY_NAMES,
    MAX_HOLD_TICKS, MAX_INPUT_TEXT_UTF16_UNITS, MAX_NEARBY_ENTITIES, MAX_WAIT_TICKS,
    MAX_WORLD_PROBE_BLOCKS, MAX_WORLD_SURFACE_COLUMNS,
};
pub use remote_host::{
    check_bind, client_host_identity, encode_client_outcome_record,
    encode_client_resource_contents, encode_client_tool_result, serve_remote, serve_remote_control,
    spawn_remote, spawn_remote_control, ClientMcpOutcome, RemoteClientHost, RemoteServeError,
    CHANGE_WATCH_INTERVAL, MAX_CONTROL_LINE_BYTES,
};

/// Why a remote-control wire payload could not be decoded.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ControlWireError {
    #[error("malformed control JSON: {0}")]
    MalformedJson(String),
    #[error("expected JSON object for {0}")]
    ExpectedObject(&'static str),
    #[error("invalid control field `{field}`: {detail}")]
    InvalidField {
        field: &'static str,
        detail: &'static str,
    },
    #[error("unknown control tunnel message type `{0}`")]
    UnknownMessageType(String),
    #[error("invalid screenshot base64 payload: {0}")]
    InvalidBase64(String),
}

/// One client control command (`method` + optional JSON `params`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlRequest {
    pub method: String,
    pub params: Value,
}

impl ControlRequest {
    pub fn new(method: impl Into<String>, params: Value) -> Self {
        Self {
            method: method.into(),
            params,
        }
    }

    pub fn to_value(&self) -> Value {
        json!({
            "method": self.method,
            "params": self.params,
        })
    }

    pub fn from_value(value: Value) -> Result<Self, ControlWireError> {
        let mut object = expect_object(value, "ControlRequest")?;
        let method = take_string(&mut object, "method")?;
        let params = object.remove("params").unwrap_or(Value::Null);
        Ok(Self { method, params })
    }

    pub fn to_json_string(&self) -> String {
        self.to_value().to_string()
    }

    pub fn to_json_vec(&self) -> Vec<u8> {
        self.to_json_string().into_bytes()
    }

    pub fn from_json_str(text: &str) -> Result<Self, ControlWireError> {
        let value: Value = serde_json::from_str(text)
            .map_err(|err| ControlWireError::MalformedJson(err.to_string()))?;
        Self::from_value(value)
    }

    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, ControlWireError> {
        let value: Value = serde_json::from_slice(bytes)
            .map_err(|err| ControlWireError::MalformedJson(err.to_string()))?;
        Self::from_value(value)
    }
}

/// Answer to one [`ControlRequest`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlResponse {
    pub ok: bool,
    pub result: Option<Value>,
    pub error: Option<String>,
}

impl ControlResponse {
    pub fn ok(result: Value) -> Self {
        Self {
            ok: true,
            result: Some(result),
            error: None,
        }
    }

    pub fn err(error: impl Into<String>) -> Self {
        Self {
            ok: false,
            result: None,
            error: Some(error.into()),
        }
    }

    pub fn to_value(&self) -> Value {
        let mut object = Map::new();
        object.insert("ok".to_string(), Value::Bool(self.ok));
        if let Some(result) = &self.result {
            object.insert("result".to_string(), result.clone());
        }
        if let Some(error) = &self.error {
            object.insert("error".to_string(), Value::String(error.clone()));
        }
        Value::Object(object)
    }

    pub fn from_value(value: Value) -> Result<Self, ControlWireError> {
        let mut object = expect_object(value, "ControlResponse")?;
        let ok = take_bool(&mut object, "ok")?;
        let result = match object.remove("result") {
            None | Some(Value::Null) => None,
            Some(other) => Some(other),
        };
        let error = match object.remove("error") {
            None | Some(Value::Null) => None,
            Some(Value::String(text)) => Some(text),
            Some(_) => {
                return Err(ControlWireError::InvalidField {
                    field: "error",
                    detail: "must be a string",
                });
            }
        };
        Ok(Self { ok, result, error })
    }

    pub fn to_json_string(&self) -> String {
        self.to_value().to_string()
    }

    pub fn to_json_vec(&self) -> Vec<u8> {
        self.to_json_string().into_bytes()
    }

    pub fn from_json_str(text: &str) -> Result<Self, ControlWireError> {
        let value: Value = serde_json::from_str(text)
            .map_err(|err| ControlWireError::MalformedJson(err.to_string()))?;
        Self::from_value(value)
    }

    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, ControlWireError> {
        let value: Value = serde_json::from_slice(bytes)
            .map_err(|err| ControlWireError::MalformedJson(err.to_string()))?;
        Self::from_value(value)
    }
}

/// Decoded screenshot bytes carried across [`RemoteControlEndpoint`] without
/// renderer crate dependencies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlScreenshot {
    pub width: u32,
    pub height: u32,
    pub mime_type: &'static str,
    pub bytes: Vec<u8>,
}

impl ControlScreenshot {
    pub fn to_tunnel(&self) -> ControlTunnelScreenshot {
        ControlTunnelScreenshot::from_screenshot(self)
    }
}

/// Base64-encoded screenshot payload sent over the `/control-ws` JSON tunnel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlTunnelScreenshot {
    pub width: u32,
    pub height: u32,
    pub mime_type: String,
    pub data_base64: String,
}

impl ControlTunnelScreenshot {
    pub fn from_screenshot(screenshot: &ControlScreenshot) -> Self {
        Self {
            width: screenshot.width,
            height: screenshot.height,
            mime_type: screenshot.mime_type.to_string(),
            data_base64: BASE64.encode(&screenshot.bytes),
        }
    }

    pub fn decode(&self) -> Result<ControlScreenshot, ControlWireError> {
        let bytes = BASE64
            .decode(self.data_base64.as_bytes())
            .map_err(|err| ControlWireError::InvalidBase64(err.to_string()))?;
        let mime_type = if self.mime_type.eq_ignore_ascii_case("image/png") {
            "image/png"
        } else if self.mime_type.eq_ignore_ascii_case("image/jpeg") {
            "image/jpeg"
        } else if self.mime_type.eq_ignore_ascii_case("image/webp") {
            "image/webp"
        } else {
            "application/octet-stream"
        };
        Ok(ControlScreenshot {
            width: self.width,
            height: self.height,
            mime_type,
            bytes,
        })
    }

    pub fn decode_lossy(&self) -> Option<ControlScreenshot> {
        self.decode().ok()
    }

    pub fn to_value(&self) -> Value {
        json!({
            "width": self.width,
            "height": self.height,
            "mime_type": self.mime_type,
            "data_base64": self.data_base64,
        })
    }

    pub fn from_value(value: Value) -> Result<Self, ControlWireError> {
        let mut object = expect_object(value, "ControlTunnelScreenshot")?;
        let width = take_u32(&mut object, "width")?;
        let height = take_u32(&mut object, "height")?;
        let mime_type = take_string(&mut object, "mime_type")?;
        let data_base64 = take_string(&mut object, "data_base64")?;
        Ok(Self {
            width,
            height,
            mime_type,
            data_base64,
        })
    }

    pub fn to_json_string(&self) -> String {
        self.to_value().to_string()
    }

    pub fn to_json_vec(&self) -> Vec<u8> {
        self.to_json_string().into_bytes()
    }

    pub fn from_json_str(text: &str) -> Result<Self, ControlWireError> {
        let value: Value = serde_json::from_str(text)
            .map_err(|err| ControlWireError::MalformedJson(err.to_string()))?;
        Self::from_value(value)
    }

    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, ControlWireError> {
        let value: Value = serde_json::from_slice(bytes)
            .map_err(|err| ControlWireError::MalformedJson(err.to_string()))?;
        Self::from_value(value)
    }
}

/// Client-to-host message on `/control-ws` (`#[serde(tag = "type", rename_all = "snake_case")]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlTunnelClientMessage {
    Hello {
        version: String,
        generation: u64,
    },
    Generation {
        generation: u64,
    },
    Response {
        id: u64,
        response: ControlResponse,
        image: Option<ControlTunnelScreenshot>,
    },
}

impl ControlTunnelClientMessage {
    pub fn to_value(&self) -> Value {
        match self {
            Self::Hello {
                version,
                generation,
            } => json!({
                "type": "hello",
                "version": version,
                "generation": generation,
            }),
            Self::Generation { generation } => json!({
                "type": "generation",
                "generation": generation,
            }),
            Self::Response {
                id,
                response,
                image,
            } => {
                let mut object = Map::new();
                object.insert("type".to_string(), Value::String("response".to_string()));
                object.insert("id".to_string(), json!(id));
                object.insert("response".to_string(), response.to_value());
                if let Some(image) = image {
                    object.insert("image".to_string(), image.to_value());
                }
                Value::Object(object)
            }
        }
    }

    pub fn from_value(value: Value) -> Result<Self, ControlWireError> {
        let mut object = expect_object(value, "ControlTunnelClientMessage")?;
        let kind = take_string(&mut object, "type")?;
        match kind.as_str() {
            "hello" => Ok(Self::Hello {
                version: take_string(&mut object, "version")?,
                generation: take_u64(&mut object, "generation")?,
            }),
            "generation" => Ok(Self::Generation {
                generation: take_u64(&mut object, "generation")?,
            }),
            "response" => {
                let id = take_u64(&mut object, "id")?;
                let response_val =
                    object
                        .remove("response")
                        .ok_or(ControlWireError::InvalidField {
                            field: "response",
                            detail: "missing field",
                        })?;
                let response = ControlResponse::from_value(response_val)?;
                let image = match object.remove("image") {
                    None | Some(Value::Null) => None,
                    Some(image_val) => Some(ControlTunnelScreenshot::from_value(image_val)?),
                };
                Ok(Self::Response {
                    id,
                    response,
                    image,
                })
            }
            _ => Err(ControlWireError::UnknownMessageType(kind)),
        }
    }

    pub fn to_json_string(&self) -> String {
        self.to_value().to_string()
    }

    pub fn to_json_vec(&self) -> Vec<u8> {
        self.to_json_string().into_bytes()
    }

    pub fn from_json_str(text: &str) -> Result<Self, ControlWireError> {
        let value: Value = serde_json::from_str(text)
            .map_err(|err| ControlWireError::MalformedJson(err.to_string()))?;
        Self::from_value(value)
    }

    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, ControlWireError> {
        let value: Value = serde_json::from_slice(bytes)
            .map_err(|err| ControlWireError::MalformedJson(err.to_string()))?;
        Self::from_value(value)
    }
}

/// Host-to-client message on `/control-ws` (`#[serde(tag = "type", rename_all = "snake_case")]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlTunnelHostMessage {
    Request {
        id: u64,
        confirmed: bool,
        request: ControlRequest,
    },
}

impl ControlTunnelHostMessage {
    pub fn to_value(&self) -> Value {
        match self {
            Self::Request {
                id,
                confirmed,
                request,
            } => json!({
                "type": "request",
                "id": id,
                "confirmed": confirmed,
                "request": request.to_value(),
            }),
        }
    }

    pub fn from_value(value: Value) -> Result<Self, ControlWireError> {
        let mut object = expect_object(value, "ControlTunnelHostMessage")?;
        let kind = take_string(&mut object, "type")?;
        match kind.as_str() {
            "request" => {
                let id = take_u64(&mut object, "id")?;
                let confirmed = take_bool(&mut object, "confirmed")?;
                let request_val =
                    object
                        .remove("request")
                        .ok_or(ControlWireError::InvalidField {
                            field: "request",
                            detail: "missing field",
                        })?;
                let request = ControlRequest::from_value(request_val)?;
                Ok(Self::Request {
                    id,
                    confirmed,
                    request,
                })
            }
            _ => Err(ControlWireError::UnknownMessageType(kind)),
        }
    }

    pub fn to_json_string(&self) -> String {
        self.to_value().to_string()
    }

    pub fn to_json_vec(&self) -> Vec<u8> {
        self.to_json_string().into_bytes()
    }

    pub fn from_json_str(text: &str) -> Result<Self, ControlWireError> {
        let value: Value = serde_json::from_str(text)
            .map_err(|err| ControlWireError::MalformedJson(err.to_string()))?;
        Self::from_value(value)
    }

    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, ControlWireError> {
        let value: Value = serde_json::from_slice(bytes)
            .map_err(|err| ControlWireError::MalformedJson(err.to_string()))?;
        Self::from_value(value)
    }
}

/// Completed remote-control call result paired with an optional decoded screenshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteControlAnswer {
    pub response: ControlResponse,
    pub image: Option<ControlScreenshot>,
}

impl RemoteControlAnswer {
    pub fn ok(result: Value) -> Self {
        Self {
            response: ControlResponse::ok(result),
            image: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            response: ControlResponse::err(message),
            image: None,
        }
    }
}

/// Transport-neutral remote client control bridge implemented by `bbb-web-host`.
pub trait RemoteControlEndpoint: Send + Sync + 'static {
    fn generation(&self) -> u64;
    fn call(
        &self,
        request: ControlRequest,
        confirmed: bool,
    ) -> Pin<Box<dyn Future<Output = RemoteControlAnswer> + Send>>;
}

fn expect_object(value: Value, name: &'static str) -> Result<Map<String, Value>, ControlWireError> {
    match value {
        Value::Object(map) => Ok(map),
        _ => Err(ControlWireError::ExpectedObject(name)),
    }
}

fn take_string(
    object: &mut Map<String, Value>,
    field: &'static str,
) -> Result<String, ControlWireError> {
    match object.remove(field) {
        Some(Value::String(text)) => Ok(text),
        None => Err(ControlWireError::InvalidField {
            field,
            detail: "missing field",
        }),
        Some(_) => Err(ControlWireError::InvalidField {
            field,
            detail: "must be a string",
        }),
    }
}

fn take_bool(
    object: &mut Map<String, Value>,
    field: &'static str,
) -> Result<bool, ControlWireError> {
    match object.remove(field) {
        Some(Value::Bool(flag)) => Ok(flag),
        None => Err(ControlWireError::InvalidField {
            field,
            detail: "missing field",
        }),
        Some(_) => Err(ControlWireError::InvalidField {
            field,
            detail: "must be a boolean",
        }),
    }
}

fn take_u64(object: &mut Map<String, Value>, field: &'static str) -> Result<u64, ControlWireError> {
    match object.remove(field) {
        Some(Value::Number(num)) => num.as_u64().ok_or(ControlWireError::InvalidField {
            field,
            detail: "must be an unsigned 64-bit integer",
        }),
        None => Err(ControlWireError::InvalidField {
            field,
            detail: "missing field",
        }),
        Some(_) => Err(ControlWireError::InvalidField {
            field,
            detail: "must be an unsigned 64-bit integer",
        }),
    }
}

fn take_u32(object: &mut Map<String, Value>, field: &'static str) -> Result<u32, ControlWireError> {
    let raw = take_u64(object, field)?;
    u32::try_from(raw).map_err(|_| ControlWireError::InvalidField {
        field,
        detail: "must fit in an unsigned 32-bit integer",
    })
}
