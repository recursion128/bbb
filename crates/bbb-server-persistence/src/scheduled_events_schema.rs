//! `TimerQueue` SavedData (`TimerQueue.java:22-45`, `:89-115`).

use bbb_protocol::{nbt_summary::entry, normalize_resource_location, JavaString, NbtSummaryValue};
use bbb_server_types::save::{
    NamedCompound, SaveResult, WorldSaveLimits, WorldSavePaths, JAVA_26_1_DATA_VERSION,
};

use crate::schema_support::{
    compound_or_empty, field, list_or_empty, long_value, require_java_26_1_root,
};

/// `TimerCallbacks.java:12-16`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Java26ScheduledCallback {
    Function(String),
    FunctionTag(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26ScheduledEvent {
    pub trigger_time: i64,
    pub id: JavaString,
    pub callback: Java26ScheduledCallback,
}

pub fn load_java_26_1_scheduled_events(
    paths: &WorldSavePaths,
    limits: WorldSaveLimits,
) -> SaveResult<Vec<Java26ScheduledEvent>> {
    let path = paths
        .saved_data_dir()
        .join("minecraft")
        .join("scheduled_events.dat");
    let Some((document, _)) = crate::level_file::read_optional_gzip_compound(&path, limits)? else {
        return Ok(Vec::new());
    };
    let data = compound_or_empty(
        require_java_26_1_root(&document, "scheduled_events")?,
        "data",
    );
    Ok(list_or_empty(data, "events")
        .iter()
        .filter_map(parse_event)
        .collect())
}

// An event the codec cannot decode is dropped (`SavedDataStorage.java:92-95`).
fn parse_event(value: &NbtSummaryValue) -> Option<Java26ScheduledEvent> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    let callback = compound_or_empty(entries, "callback");
    let identifier = |name: &str| match field(callback, name)? {
        NbtSummaryValue::String(id) => id
            .try_to_string()
            .ok()
            .and_then(|raw| normalize_resource_location(raw).ok()),
        _ => None,
    };
    let callback_id = identifier("id")?;
    let callback = match identifier("type")?.as_str() {
        "minecraft:function" => Java26ScheduledCallback::Function(callback_id),
        "minecraft:function_tag" => Java26ScheduledCallback::FunctionTag(callback_id),
        _ => return None,
    };
    Some(Java26ScheduledEvent {
        trigger_time: field(entries, "trigger_time").and_then(long_value)?,
        id: JavaString::from_nbt(field(entries, "id")?)?,
        callback,
    })
}

/// `TimerQueue.java:89-115`; `SavedDataStorage.java:198-204` wraps its codec.
pub fn java_26_1_scheduled_events_document(events: &[Java26ScheduledEvent]) -> NamedCompound {
    let events = events
        .iter()
        .map(|event| {
            let (kind, id) = match &event.callback {
                Java26ScheduledCallback::Function(id) => ("minecraft:function", id),
                Java26ScheduledCallback::FunctionTag(id) => ("minecraft:function_tag", id),
            };
            NbtSummaryValue::Compound(vec![
                entry("trigger_time", NbtSummaryValue::Long(event.trigger_time)),
                entry("id", event.id.to_nbt()),
                entry(
                    "callback",
                    NbtSummaryValue::Compound(vec![
                        entry("type", NbtSummaryValue::String(kind.into())),
                        entry("id", NbtSummaryValue::string(id.clone())),
                    ]),
                ),
            ])
        })
        .collect();
    NamedCompound::new(
        "",
        vec![
            entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
            entry(
                "data",
                NbtSummaryValue::Compound(vec![entry("events", NbtSummaryValue::List(events))]),
            ),
        ],
    )
}
