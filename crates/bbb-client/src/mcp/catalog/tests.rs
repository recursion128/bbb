use std::sync::{Arc, Mutex};

use bbb_mcp::control::tool_by_name;
use serde_json::json;

use crate::control::{
    dispatch, ControlRequest, ControlRequestSink, ControlState, ControlSubmitError,
    ControlSubmitReceipt, ControlWriteRequest, InputControlRequest, InputKeyControl,
    SharedControlRequests,
};

#[derive(Default)]
struct InputSink(Mutex<Vec<ControlWriteRequest>>);

impl ControlRequestSink for InputSink {
    fn submit(
        &self,
        request: ControlWriteRequest,
    ) -> Result<ControlSubmitReceipt, ControlSubmitError> {
        let mut requests = self.0.lock().unwrap();
        requests.push(request);
        Ok(ControlSubmitReceipt {
            pending: requests.len(),
        })
    }
}

fn state() -> (ControlState, Arc<InputSink>) {
    let sink = Arc::new(InputSink::default());
    let state = ControlState::new("test", SharedControlRequests::new(sink.clone()));
    (state, sink)
}

#[test]
fn advertised_keys_reach_the_input_owner_with_their_recording_names() {
    let schema = tool_by_name("input.key").unwrap().descriptor().input_schema;
    let keys = schema["properties"]["key"]["enum"].as_array().unwrap();
    let (state, sink) = state();
    for name in keys {
        let key: InputKeyControl = serde_json::from_value(name.clone()).unwrap();
        assert_eq!(serde_json::to_value(key).unwrap(), *name);
        let response = dispatch(
            ControlRequest {
                method: "input.key".to_string(),
                params: json!({"key": name, "pressed": true}),
            },
            &state,
        );
        assert!(response.ok, "{name}: {:?}", response.error);
        assert_eq!(
            sink.0.lock().unwrap().pop(),
            Some(ControlWriteRequest::Input(InputControlRequest::Key {
                key,
                pressed: true,
            }))
        );
    }
    for name in [
        "f1",
        "f3",
        "f4",
        "f5",
        "f6",
        "tab",
        "drop",
        "swap_offhand",
        "advancements",
        "social",
    ] {
        assert!(keys.contains(&json!(name)), "missing {name}");
    }
}

#[test]
fn text_schema_explains_the_utf16_budget_enforced_by_control() {
    let schema = tool_by_name("input.text")
        .unwrap()
        .descriptor()
        .input_schema;
    let text_schema = &schema["properties"]["text"];
    assert_eq!(
        text_schema["maxLength"],
        InputControlRequest::MAX_TEXT_UTF16_UNITS
    );
    assert!(text_schema["description"]
        .as_str()
        .unwrap()
        .contains("UTF-16"));
    let (state, sink) = state();
    let text = "😀".repeat(InputControlRequest::MAX_TEXT_UTF16_UNITS / 2);
    let request = |text: &str| ControlRequest {
        method: "input.text".to_string(),
        params: json!({"text": text}),
    };
    assert!(dispatch(request(&text), &state).ok);
    assert!(!dispatch(request(&(text.clone() + "x")), &state).ok);
    assert_eq!(
        sink.0.lock().unwrap().as_slice(),
        &[ControlWriteRequest::Input(InputControlRequest::Text {
            text
        })]
    );
}

#[test]
fn perception_query_budgets_and_crosshair_resource_match_control_surface() {
    assert_eq!(
        bbb_mcp::control::MAX_WORLD_PROBE_BLOCKS,
        crate::control::MAX_WORLD_PROBE_BLOCKS
    );
    assert_eq!(
        bbb_mcp::control::MAX_WORLD_SURFACE_COLUMNS,
        crate::control::MAX_WORLD_SURFACE_COLUMNS
    );
    assert_eq!(
        bbb_mcp::control::MAX_NEARBY_ENTITIES,
        crate::control::MAX_NEARBY_ENTITIES
    );
    let blocks_schema = tool_by_name("world.probe_blocks")
        .unwrap()
        .descriptor()
        .input_schema;
    assert_eq!(
        blocks_schema["properties"]["positions"]["maxItems"],
        crate::control::MAX_WORLD_PROBE_BLOCKS
    );
    let nearby_schema = tool_by_name("world.nearby_entities")
        .unwrap()
        .descriptor()
        .input_schema;
    assert_eq!(
        nearby_schema["properties"]["limit"]["maximum"],
        crate::control::MAX_NEARBY_ENTITIES
    );
    let crosshair = tool_by_name("world.crosshair_target").unwrap();
    assert_eq!(
        crosshair.resource_uri,
        Some("bbb://client/world/crosshair_target")
    );
    assert_eq!(
        bbb_mcp::control::MAX_HOLD_TICKS,
        crate::control::MAX_HOLD_TICKS
    );
    assert_eq!(
        bbb_mcp::control::MAX_WAIT_TICKS,
        crate::control::MAX_WAIT_TICKS
    );
    for tool_name in [
        "input.look",
        "input.look_at",
        "input.scroll",
        "sim.wait_ticks",
    ] {
        assert!(tool_by_name(tool_name).is_some(), "missing {tool_name}");
    }
}
