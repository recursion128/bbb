use bbb_protocol::{
    game_rules::{vanilla_game_rule, GameRuleValueKind},
    NbtSummaryEntry, NbtSummaryValue,
};

use crate::{
    FrozenRegistryAccess, FrozenRegistryEntry, RegistryElementSource, RegistryEntryPayload,
    RegistryEntrySource, WorldStore,
};

use super::normalize_codec_identifier;

pub const TEST_ENVIRONMENT_REGISTRY: &str = "minecraft:test_environment";
pub const TEST_INSTANCE_REGISTRY: &str = "minecraft:test_instance";
const WORLD_CLOCK_REGISTRY: &str = "minecraft:world_clock";
const TIMELINE_REGISTRY: &str = "minecraft:timeline";
const MAX_DIRECT_ENVIRONMENT_DEPTH: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestRegistryReference {
    pub holder_id: i32,
    pub resource_id: String,
    pub source: RegistryEntrySource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestEnvironmentHolder {
    Named(TestRegistryReference),
    Direct(Box<TestEnvironmentDefinition>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestGameRuleValue {
    pub resource_id: String,
    pub value: TestGameRuleTypedValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestGameRuleTypedValue {
    Boolean(bool),
    Integer(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestWeather {
    Clear,
    Rain,
    Thunder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestEnvironmentKind {
    AllOf {
        definitions: Vec<TestEnvironmentHolder>,
    },
    GameRules {
        rules: Vec<TestGameRuleValue>,
    },
    ClockTime {
        clock: TestRegistryReference,
        time: i32,
    },
    TimelineAttributes {
        timelines: Vec<TestRegistryReference>,
    },
    Weather {
        weather: TestWeather,
    },
    Functions {
        setup: Option<String>,
        teardown: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestEnvironmentDefinition {
    pub element_source: RegistryElementSource,
    pub kind: TestEnvironmentKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestRotation {
    None,
    Clockwise90,
    Clockwise180,
    Counterclockwise90,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameTestInstanceKind {
    BlockBased,
    Function { function: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameTestInstanceDefinition {
    pub element_source: RegistryElementSource,
    pub kind: GameTestInstanceKind,
    pub environment: TestEnvironmentHolder,
    pub structure: String,
    pub max_ticks: i32,
    pub setup_ticks: i32,
    pub required: bool,
    pub rotation: TestRotation,
    pub manual_only: bool,
    pub max_attempts: i32,
    pub required_successes: i32,
    pub sky_access: bool,
    pub padding: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameTestRegistryProjectionError {
    pub resource_id: String,
    pub field: String,
    pub reason: String,
}

impl std::fmt::Display for GameTestRegistryProjectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} {}: {}",
            self.resource_id, self.field, self.reason
        )
    }
}

impl std::error::Error for GameTestRegistryProjectionError {}

impl WorldStore {
    pub fn synchronized_test_environment_definition(
        &self,
        holder_id: i32,
    ) -> Result<Option<TestEnvironmentDefinition>, GameTestRegistryProjectionError> {
        let access = self.frozen_registry_access();
        let Some(entry) = access.entry_by_holder_id(TEST_ENVIRONMENT_REGISTRY, holder_id) else {
            return Ok(None);
        };
        if let Some(definition) = self
            .registries
            .typed
            .test_environments
            .get(entry.resource_id)
        {
            return Ok(Some(definition.clone()));
        }
        let Some(data) = entry.element_data else {
            if entry_requires_typed_payload(entry) {
                return Err(error(
                    entry.resource_id,
                    "<root>",
                    "typed payload unavailable",
                ));
            }
            return Ok(None);
        };
        decode_test_environment(
            entry.resource_id,
            data.source,
            &data.value,
            data.known_pack_json.as_ref(),
            access,
            0,
            "<root>",
        )
        .map(Some)
    }

    pub fn synchronized_test_instance_definition(
        &self,
        holder_id: i32,
    ) -> Result<Option<GameTestInstanceDefinition>, GameTestRegistryProjectionError> {
        let access = self.frozen_registry_access();
        let Some(entry) = access.entry_by_holder_id(TEST_INSTANCE_REGISTRY, holder_id) else {
            return Ok(None);
        };
        if let Some(definition) = self.registries.typed.test_instances.get(entry.resource_id) {
            return Ok(Some(definition.clone()));
        }
        let Some(data) = entry.element_data else {
            if entry_requires_typed_payload(entry) {
                return Err(error(
                    entry.resource_id,
                    "<root>",
                    "typed payload unavailable",
                ));
            }
            return Ok(None);
        };
        decode_test_instance(
            entry.resource_id,
            data.source,
            &data.value,
            data.known_pack_json.as_ref(),
            access,
        )
        .map(Some)
    }
}

fn entry_requires_typed_payload(entry: FrozenRegistryEntry<'_>) -> bool {
    entry.element_data_error.is_some()
        || matches!(
            entry.payload,
            RegistryEntryPayload::Server { has_data: true, .. }
                | RegistryEntryPayload::Server {
                    known_pack_data: Some(_),
                    ..
                }
        )
}

pub(super) fn decode_test_environment(
    resource_id: &str,
    element_source: RegistryElementSource,
    value: &NbtSummaryValue,
    known_pack_json: Option<&serde_json::Value>,
    access: FrozenRegistryAccess<'_>,
    depth: usize,
    path: &str,
) -> Result<TestEnvironmentDefinition, GameTestRegistryProjectionError> {
    if depth > MAX_DIRECT_ENVIRONMENT_DEPTH {
        return Err(error(
            resource_id,
            path,
            format!("direct environment nesting exceeds {MAX_DIRECT_ENVIRONMENT_DEPTH}"),
        ));
    }
    let fields = compound(value, resource_id, path)?;
    let type_path = child(path, "type");
    let definition_type = required_identifier(fields, resource_id, &type_path)?;
    let kind = match definition_type.as_str() {
        "minecraft:all_of" => {
            let definitions_path = child(path, "definitions");
            let values = list(
                required_field(fields, resource_id, &definitions_path)?,
                resource_id,
                &definitions_path,
            )?;
            let mut definitions = Vec::with_capacity(values.len());
            let json_definitions = known_pack_json
                .and_then(serde_json::Value::as_object)
                .and_then(|fields| fields.get("definitions"))
                .and_then(serde_json::Value::as_array);
            for (index, definition) in values.iter().enumerate() {
                definitions.push(decode_environment_holder(
                    definition,
                    json_definitions.and_then(|definitions| definitions.get(index)),
                    resource_id,
                    element_source,
                    access,
                    depth + 1,
                    &format!("{definitions_path}[{index}]"),
                )?);
            }
            TestEnvironmentKind::AllOf { definitions }
        }
        "minecraft:game_rules" => {
            let rules_path = child(path, "rules");
            let rule_fields = compound(
                required_field(fields, resource_id, &rules_path)?,
                resource_id,
                &rules_path,
            )?;
            let mut rules = Vec::with_capacity(rule_fields.len());
            let json_rules = known_pack_json
                .and_then(serde_json::Value::as_object)
                .and_then(|fields| fields.get("rules"))
                .and_then(serde_json::Value::as_object);
            for rule in rule_fields {
                let rule_id = normalize_codec_identifier(&rule.name).ok_or_else(|| {
                    error(
                        resource_id,
                        format!("{rules_path}.{}", rule.name),
                        "invalid game-rule identifier",
                    )
                })?;
                let value_path = format!("{rules_path}.{rule_id}");
                let value = decode_game_rule_value(
                    resource_id,
                    &rule_id,
                    &rule.value,
                    json_rules.and_then(|rules| rules.get(&rule.name)),
                    &value_path,
                )?;
                rules.push(TestGameRuleValue {
                    resource_id: rule_id,
                    value,
                });
            }
            TestEnvironmentKind::GameRules { rules }
        }
        "minecraft:clock_time" => {
            let clock_path = child(path, "clock");
            let clock = decode_named_reference(
                required_field(fields, resource_id, &clock_path)?,
                resource_id,
                WORLD_CLOCK_REGISTRY,
                access,
                &clock_path,
            )?;
            let time_path = child(path, "time");
            let time = required_i32(fields, resource_id, &time_path)?;
            if time < 0 {
                return Err(error(
                    resource_id,
                    time_path,
                    "expected non-negative integer",
                ));
            }
            TestEnvironmentKind::ClockTime { clock, time }
        }
        "minecraft:timeline_attributes" => {
            let timelines_path = child(path, "timelines");
            let values = list(
                required_field(fields, resource_id, &timelines_path)?,
                resource_id,
                &timelines_path,
            )?;
            let mut timelines = Vec::with_capacity(values.len());
            for (index, timeline) in values.iter().enumerate() {
                timelines.push(decode_named_reference(
                    timeline,
                    resource_id,
                    TIMELINE_REGISTRY,
                    access,
                    &format!("{timelines_path}[{index}]"),
                )?);
            }
            TestEnvironmentKind::TimelineAttributes { timelines }
        }
        "minecraft:weather" => {
            let weather_path = child(path, "weather");
            let weather = match required_string(fields, resource_id, &weather_path)? {
                "clear" => TestWeather::Clear,
                "rain" => TestWeather::Rain,
                "thunder" => TestWeather::Thunder,
                value => {
                    return Err(error(
                        resource_id,
                        weather_path,
                        format!("unknown weather {value}"),
                    ))
                }
            };
            TestEnvironmentKind::Weather { weather }
        }
        "minecraft:function" => TestEnvironmentKind::Functions {
            setup: optional_identifier(fields, resource_id, &child(path, "setup"))?,
            teardown: optional_identifier(fields, resource_id, &child(path, "teardown"))?,
        },
        value => {
            return Err(error(
                resource_id,
                type_path,
                format!("unknown test-environment type {value}"),
            ))
        }
    };
    Ok(TestEnvironmentDefinition {
        element_source,
        kind,
    })
}

fn decode_game_rule_value(
    owner_resource_id: &str,
    rule_id: &str,
    value: &NbtSummaryValue,
    known_pack_json: Option<&serde_json::Value>,
    path: &str,
) -> Result<TestGameRuleTypedValue, GameTestRegistryProjectionError> {
    let rule = vanilla_game_rule(rule_id).ok_or_else(|| {
        error(
            owner_resource_id,
            path,
            format!("unknown built-in game rule {rule_id}"),
        )
    })?;
    match rule.value_kind {
        GameRuleValueKind::Boolean => match (value, known_pack_json) {
            (_, Some(serde_json::Value::Bool(value))) => {
                Ok(TestGameRuleTypedValue::Boolean(*value))
            }
            (_, Some(_)) => Err(error(owner_resource_id, path, "expected boolean value")),
            (NbtSummaryValue::Long(0), None) => Ok(TestGameRuleTypedValue::Boolean(false)),
            (NbtSummaryValue::Long(1), None) => Ok(TestGameRuleTypedValue::Boolean(true)),
            _ => Err(error(owner_resource_id, path, "expected boolean value")),
        },
        GameRuleValueKind::Integer { min, max } => {
            if known_pack_json.is_some_and(|value| !value.is_i64() && !value.is_u64()) {
                return Err(error(
                    owner_resource_id,
                    path,
                    format!("expected integer in range {min}..={max}"),
                ));
            }
            let NbtSummaryValue::Long(value) = value else {
                return Err(error(
                    owner_resource_id,
                    path,
                    format!("expected integer in range {min}..={max}"),
                ));
            };
            let value = i32::try_from(*value).map_err(|_| {
                error(
                    owner_resource_id,
                    path,
                    format!("expected integer in range {min}..={max}"),
                )
            })?;
            if !(min..=max).contains(&value) {
                return Err(error(
                    owner_resource_id,
                    path,
                    format!("expected integer in range {min}..={max}"),
                ));
            }
            Ok(TestGameRuleTypedValue::Integer(value))
        }
    }
}

fn decode_environment_holder(
    value: &NbtSummaryValue,
    known_pack_json: Option<&serde_json::Value>,
    resource_id: &str,
    element_source: RegistryElementSource,
    access: FrozenRegistryAccess<'_>,
    depth: usize,
    path: &str,
) -> Result<TestEnvironmentHolder, GameTestRegistryProjectionError> {
    match value {
        NbtSummaryValue::String(_) => Ok(TestEnvironmentHolder::Named(decode_named_reference(
            value,
            resource_id,
            TEST_ENVIRONMENT_REGISTRY,
            access,
            path,
        )?)),
        NbtSummaryValue::Compound(_) => Ok(TestEnvironmentHolder::Direct(Box::new(
            decode_test_environment(
                resource_id,
                element_source,
                value,
                known_pack_json,
                access,
                depth,
                path,
            )?,
        ))),
        _ => Err(error(
            resource_id,
            path,
            "environment holder is neither a resource id nor a direct compound",
        )),
    }
}

pub(super) fn decode_test_instance(
    resource_id: &str,
    element_source: RegistryElementSource,
    value: &NbtSummaryValue,
    known_pack_json: Option<&serde_json::Value>,
    access: FrozenRegistryAccess<'_>,
) -> Result<GameTestInstanceDefinition, GameTestRegistryProjectionError> {
    let path = "<root>";
    let fields = compound(value, resource_id, path)?;
    let type_path = child(path, "type");
    let instance_type = required_identifier(fields, resource_id, &type_path)?;
    let kind = match instance_type.as_str() {
        "minecraft:block_based" => GameTestInstanceKind::BlockBased,
        "minecraft:function" => {
            let function_path = child(path, "function");
            GameTestInstanceKind::Function {
                function: required_identifier(fields, resource_id, &function_path)?,
            }
        }
        value => {
            return Err(error(
                resource_id,
                type_path,
                format!("unknown test-instance type {value}"),
            ))
        }
    };
    let environment_path = child(path, "environment");
    let environment = decode_environment_holder(
        required_field(fields, resource_id, &environment_path)?,
        known_pack_json
            .and_then(serde_json::Value::as_object)
            .and_then(|fields| fields.get("environment")),
        resource_id,
        element_source,
        access,
        0,
        &environment_path,
    )?;
    let structure_path = child(path, "structure");
    let structure = required_identifier(fields, resource_id, &structure_path)?;
    let max_ticks_path = child(path, "max_ticks");
    let max_ticks = required_positive_i32(fields, resource_id, &max_ticks_path)?;
    let setup_ticks = optional_non_negative_i32(fields, resource_id, path, "setup_ticks", 0)?;
    let required = optional_bool(fields, resource_id, path, "required", true)?;
    let rotation_path = child(path, "rotation");
    let rotation = match optional_string(fields, resource_id, &rotation_path)?.unwrap_or("none") {
        "none" => TestRotation::None,
        "clockwise_90" => TestRotation::Clockwise90,
        "180" => TestRotation::Clockwise180,
        "counterclockwise_90" => TestRotation::Counterclockwise90,
        value => {
            return Err(error(
                resource_id,
                rotation_path,
                format!("unknown rotation {value}"),
            ))
        }
    };
    let manual_only = optional_bool(fields, resource_id, path, "manual_only", false)?;
    let max_attempts = optional_positive_i32(fields, resource_id, path, "max_attempts", 1)?;
    let required_successes =
        optional_positive_i32(fields, resource_id, path, "required_successes", 1)?;
    let sky_access = optional_bool(fields, resource_id, path, "sky_access", false)?;
    let padding = optional_non_negative_i32(fields, resource_id, path, "padding", 0)?;
    if padding > 128 {
        return Err(error(
            resource_id,
            child(path, "padding"),
            "integer outside 0..=128",
        ));
    }
    Ok(GameTestInstanceDefinition {
        element_source,
        kind,
        environment,
        structure,
        max_ticks,
        setup_ticks,
        required,
        rotation,
        manual_only,
        max_attempts,
        required_successes,
        sky_access,
        padding,
    })
}

fn decode_named_reference(
    value: &NbtSummaryValue,
    resource_id: &str,
    registry: &str,
    access: FrozenRegistryAccess<'_>,
    path: &str,
) -> Result<TestRegistryReference, GameTestRegistryProjectionError> {
    let NbtSummaryValue::String(reference) = value else {
        return Err(error(resource_id, path, "expected resource identifier"));
    };
    let reference = normalize_codec_identifier(reference)
        .ok_or_else(|| error(resource_id, path, "invalid resource identifier"))?;
    let entry = access
        .entry_by_resource_id(registry, &reference)
        .ok_or_else(|| {
            error(
                resource_id,
                path,
                format!("{reference} is absent from current session {registry}"),
            )
        })?;
    Ok(TestRegistryReference {
        holder_id: entry.holder_id,
        resource_id: entry.resource_id.to_string(),
        source: entry.source,
    })
}

fn compound<'a>(
    value: &'a NbtSummaryValue,
    resource_id: &str,
    path: &str,
) -> Result<&'a [NbtSummaryEntry], GameTestRegistryProjectionError> {
    match value {
        NbtSummaryValue::Compound(fields) => Ok(fields),
        _ => Err(error(resource_id, path, "expected compound")),
    }
}

fn list<'a>(
    value: &'a NbtSummaryValue,
    resource_id: &str,
    path: &str,
) -> Result<&'a [NbtSummaryValue], GameTestRegistryProjectionError> {
    match value {
        NbtSummaryValue::List(values) => Ok(values),
        _ => Err(error(resource_id, path, "expected list")),
    }
}

fn field<'a>(fields: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    fields
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn field_name(path: &str) -> &str {
    path.rsplit('.').next().unwrap_or(path)
}

fn required_field<'a>(
    fields: &'a [NbtSummaryEntry],
    resource_id: &str,
    path: &str,
) -> Result<&'a NbtSummaryValue, GameTestRegistryProjectionError> {
    field(fields, field_name(path))
        .ok_or_else(|| error(resource_id, path, "missing required field"))
}

fn required_string<'a>(
    fields: &'a [NbtSummaryEntry],
    resource_id: &str,
    path: &str,
) -> Result<&'a str, GameTestRegistryProjectionError> {
    match required_field(fields, resource_id, path)? {
        NbtSummaryValue::String(value) => Ok(value),
        _ => Err(error(resource_id, path, "expected string")),
    }
}

fn optional_string<'a>(
    fields: &'a [NbtSummaryEntry],
    resource_id: &str,
    path: &str,
) -> Result<Option<&'a str>, GameTestRegistryProjectionError> {
    match field(fields, field_name(path)) {
        Some(NbtSummaryValue::String(value)) => Ok(Some(value)),
        Some(_) => Err(error(resource_id, path, "expected string")),
        None => Ok(None),
    }
}

fn required_identifier(
    fields: &[NbtSummaryEntry],
    resource_id: &str,
    path: &str,
) -> Result<String, GameTestRegistryProjectionError> {
    let value = required_string(fields, resource_id, path)?;
    normalize_codec_identifier(value)
        .ok_or_else(|| error(resource_id, path, "invalid resource identifier"))
}

fn optional_identifier(
    fields: &[NbtSummaryEntry],
    resource_id: &str,
    path: &str,
) -> Result<Option<String>, GameTestRegistryProjectionError> {
    optional_string(fields, resource_id, path)?
        .map(|value| {
            normalize_codec_identifier(value)
                .ok_or_else(|| error(resource_id, path, "invalid resource identifier"))
        })
        .transpose()
}

fn required_i32(
    fields: &[NbtSummaryEntry],
    resource_id: &str,
    path: &str,
) -> Result<i32, GameTestRegistryProjectionError> {
    numeric_i32(required_field(fields, resource_id, path)?)
        .ok_or_else(|| error(resource_id, path, "expected integer"))
}

fn required_positive_i32(
    fields: &[NbtSummaryEntry],
    resource_id: &str,
    path: &str,
) -> Result<i32, GameTestRegistryProjectionError> {
    let value = required_i32(fields, resource_id, path)?;
    if value > 0 {
        Ok(value)
    } else {
        Err(error(resource_id, path, "expected positive integer"))
    }
}

fn optional_non_negative_i32(
    fields: &[NbtSummaryEntry],
    resource_id: &str,
    parent: &str,
    name: &str,
    default: i32,
) -> Result<i32, GameTestRegistryProjectionError> {
    let path = child(parent, name);
    let Some(value) = field(fields, name) else {
        return Ok(default);
    };
    let value = numeric_i32(value).ok_or_else(|| error(resource_id, &path, "expected integer"))?;
    if value >= 0 {
        Ok(value)
    } else {
        Err(error(resource_id, path, "expected non-negative integer"))
    }
}

fn optional_positive_i32(
    fields: &[NbtSummaryEntry],
    resource_id: &str,
    parent: &str,
    name: &str,
    default: i32,
) -> Result<i32, GameTestRegistryProjectionError> {
    let path = child(parent, name);
    let Some(value) = field(fields, name) else {
        return Ok(default);
    };
    let value = numeric_i32(value).ok_or_else(|| error(resource_id, &path, "expected integer"))?;
    if value > 0 {
        Ok(value)
    } else {
        Err(error(resource_id, path, "expected positive integer"))
    }
}

fn optional_bool(
    fields: &[NbtSummaryEntry],
    resource_id: &str,
    parent: &str,
    name: &str,
    default: bool,
) -> Result<bool, GameTestRegistryProjectionError> {
    let path = child(parent, name);
    match field(fields, name) {
        Some(NbtSummaryValue::Byte(0)) => Ok(false),
        Some(NbtSummaryValue::Byte(1)) => Ok(true),
        Some(_) => Err(error(resource_id, path, "expected boolean")),
        None => Ok(default),
    }
}

fn numeric_i32(value: &NbtSummaryValue) -> Option<i32> {
    match value {
        NbtSummaryValue::Byte(value) => Some(i32::from(*value)),
        NbtSummaryValue::Short(value) => Some(i32::from(*value)),
        NbtSummaryValue::Int(value) => Some(*value),
        NbtSummaryValue::Long(value) => i32::try_from(*value).ok(),
        _ => None,
    }
}

fn child(parent: &str, name: &str) -> String {
    if parent == "<root>" {
        name.to_string()
    } else {
        format!("{parent}.{name}")
    }
}

fn error(
    resource_id: impl Into<String>,
    field: impl Into<String>,
    reason: impl Into<String>,
) -> GameTestRegistryProjectionError {
    GameTestRegistryProjectionError {
        resource_id: resource_id.into(),
        field: field.into(),
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegistryPacketEntry;

    #[test]
    fn projects_known_pack_environment_and_instance_with_session_references() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            TEST_ENVIRONMENT_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::with_known_pack_data_for_test(
                    "example:weather",
                    br#"{"type":"minecraft:weather","weather":"thunder"}"#.to_vec(),
                ),
                RegistryPacketEntry::with_known_pack_data_for_test(
                    "example:combined",
                    br#"{
                        "type":"minecraft:all_of",
                        "definitions":[
                            "example:weather",
                            {"type":"minecraft:function","setup":"example:setup"}
                        ]
                    }"#
                    .to_vec(),
                ),
            ],
        );
        store.record_registry_entries(
            TEST_INSTANCE_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_known_pack_data_for_test(
                "example:instance",
                br#"{
                    "type":"minecraft:function",
                    "environment":"example:combined",
                    "function":"example:run",
                    "structure":"example:structure",
                    "max_ticks":40,
                    "rotation":"clockwise_90",
                    "max_attempts":3,
                    "required_successes":2,
                    "padding":16
                }"#
                .to_vec(),
            )],
        );

        let combined = store
            .synchronized_test_environment_definition(1)
            .unwrap()
            .unwrap();
        assert_eq!(
            combined.element_source,
            RegistryElementSource::KnownPackJson
        );
        let TestEnvironmentKind::AllOf { definitions } = combined.kind else {
            panic!("expected all_of environment")
        };
        assert_eq!(definitions.len(), 2);
        assert!(matches!(
            &definitions[0],
            TestEnvironmentHolder::Named(reference)
                if reference.holder_id == 0 && reference.resource_id == "example:weather"
        ));
        assert!(matches!(
            &definitions[1],
            TestEnvironmentHolder::Direct(definition)
                if matches!(
                    &definition.kind,
                    TestEnvironmentKind::Functions { setup: Some(setup), teardown: None }
                        if setup == "example:setup"
                )
        ));

        let instance = store
            .synchronized_test_instance_definition(0)
            .unwrap()
            .unwrap();
        assert_eq!(
            instance.element_source,
            RegistryElementSource::KnownPackJson
        );
        assert_eq!(
            instance.kind,
            GameTestInstanceKind::Function {
                function: "example:run".to_string(),
            }
        );
        assert!(matches!(
            instance.environment,
            TestEnvironmentHolder::Named(reference)
                if reference.holder_id == 1 && reference.resource_id == "example:combined"
        ));
        assert_eq!(instance.setup_ticks, 0);
        assert!(instance.required);
        assert_eq!(instance.rotation, TestRotation::Clockwise90);
        assert_eq!(instance.max_attempts, 3);
        assert_eq!(instance.required_successes, 2);
        assert_eq!(instance.padding, 16);
    }

    #[test]
    fn projects_network_environment_and_rejects_unknown_or_invalid_fields() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            TEST_ENVIRONMENT_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::with_raw_data("example:rain", weather_environment_nbt("rain")),
                RegistryPacketEntry::with_known_pack_data_for_test(
                    "example:broken",
                    br#"{"type":"minecraft:weather","weather":"snow"}"#.to_vec(),
                ),
            ],
        );

        let rain = store
            .synchronized_test_environment_definition(0)
            .unwrap()
            .unwrap();
        assert_eq!(rain.element_source, RegistryElementSource::NetworkNbt);
        assert_eq!(
            rain.kind,
            TestEnvironmentKind::Weather {
                weather: TestWeather::Rain,
            }
        );
        assert_eq!(
            store
                .synchronized_test_environment_definition(1)
                .unwrap_err(),
            error("example:broken", "weather", "unknown weather snow")
        );

        store.record_registry_entries(
            TEST_INSTANCE_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_known_pack_data_for_test(
                "example:broken_instance",
                br#"{
                    "type":"minecraft:block_based",
                    "environment":"example:missing",
                    "structure":"example:test",
                    "max_ticks":0
                }"#
                .to_vec(),
            )],
        );
        assert!(store
            .synchronized_test_instance_definition(0)
            .unwrap_err()
            .reason
            .contains("absent from current session"));
    }

    #[test]
    fn projects_remaining_environment_variants_and_block_based_defaults() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            WORLD_CLOCK_REGISTRY,
            0,
            vec![RegistryPacketEntry::stub("example:clock")],
        );
        store.record_registry_entries(
            TIMELINE_REGISTRY,
            0,
            vec![RegistryPacketEntry::stub("example:timeline")],
        );
        store.record_registry_entries(
            TEST_ENVIRONMENT_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::with_known_pack_data_for_test(
                    "example:rules",
                    br#"{
                        "type":"minecraft:game_rules",
                        "rules":{
                            "minecraft:keep_inventory":true,
                            "minecraft:random_tick_speed":7
                        }
                    }"#
                    .to_vec(),
                ),
                RegistryPacketEntry::with_known_pack_data_for_test(
                    "example:clock_time",
                    br#"{
                        "type":"minecraft:clock_time",
                        "clock":"example:clock",
                        "time":6000
                    }"#
                    .to_vec(),
                ),
                RegistryPacketEntry::with_known_pack_data_for_test(
                    "example:timelines",
                    br#"{
                        "type":"minecraft:timeline_attributes",
                        "timelines":["example:timeline"]
                    }"#
                    .to_vec(),
                ),
            ],
        );

        let rules = store
            .synchronized_test_environment_definition(0)
            .unwrap()
            .unwrap();
        assert!(matches!(
            rules.kind,
            TestEnvironmentKind::GameRules { rules }
                if rules.len() == 2
                    && rules[0].resource_id == "minecraft:keep_inventory"
                    && rules[0].value == TestGameRuleTypedValue::Boolean(true)
                    && rules[1].resource_id == "minecraft:random_tick_speed"
                    && rules[1].value == TestGameRuleTypedValue::Integer(7)
        ));
        let clock_time = store
            .synchronized_test_environment_definition(1)
            .unwrap()
            .unwrap();
        assert!(matches!(
            clock_time.kind,
            TestEnvironmentKind::ClockTime { clock, time }
                if clock.holder_id == 0 && clock.resource_id == "example:clock" && time == 6000
        ));
        let timelines = store
            .synchronized_test_environment_definition(2)
            .unwrap()
            .unwrap();
        assert!(matches!(
            timelines.kind,
            TestEnvironmentKind::TimelineAttributes { timelines }
                if timelines.len() == 1
                    && timelines[0].holder_id == 0
                    && timelines[0].resource_id == "example:timeline"
        ));

        store.record_registry_entries(
            TEST_INSTANCE_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_known_pack_data_for_test(
                "example:block_test",
                br#"{
                    "type":"minecraft:block_based",
                    "environment":{"type":"minecraft:weather","weather":"clear"},
                    "structure":"example:block_test",
                    "max_ticks":20
                }"#
                .to_vec(),
            )],
        );
        let instance = store
            .synchronized_test_instance_definition(0)
            .unwrap()
            .unwrap();
        assert_eq!(instance.kind, GameTestInstanceKind::BlockBased);
        assert!(matches!(
            instance.environment,
            TestEnvironmentHolder::Direct(_)
        ));
        assert_eq!(instance.setup_ticks, 0);
        assert!(instance.required);
        assert_eq!(instance.rotation, TestRotation::None);
        assert!(!instance.manual_only);
        assert_eq!(instance.max_attempts, 1);
        assert_eq!(instance.required_successes, 1);
        assert!(!instance.sky_access);
        assert_eq!(instance.padding, 0);
    }

    #[test]
    fn network_nbt_game_rules_accept_codec_bool_and_integer_shapes() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            TEST_ENVIRONMENT_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_raw_data(
                "example:network_rules",
                game_rules_environment_nbt(true, 7),
            )],
        );

        let definition = store
            .synchronized_test_environment_definition(0)
            .unwrap()
            .unwrap();
        assert!(matches!(
            definition.kind,
            TestEnvironmentKind::GameRules { rules }
                if rules.iter().map(|rule| rule.value).collect::<Vec<_>>()
                    == vec![
                        TestGameRuleTypedValue::Boolean(true),
                        TestGameRuleTypedValue::Integer(7),
                    ]
        ));
    }

    #[test]
    fn game_rule_codec_dispatch_rejects_unknown_wrong_type_and_out_of_range_values() {
        let cases = [
            (
                br#"{"minecraft:custom_rule":true}"#.as_slice(),
                "unknown built-in game rule minecraft:custom_rule",
            ),
            (
                br#"{"minecraft:keep_inventory":2}"#.as_slice(),
                "expected boolean value",
            ),
            (
                br#"{"minecraft:random_tick_speed":true}"#.as_slice(),
                "expected integer in range 0..=2147483647",
            ),
            (
                br#"{"minecraft:max_snow_accumulation_height":9}"#.as_slice(),
                "expected integer in range 0..=8",
            ),
        ];

        for (rules, expected) in cases {
            let mut payload = br#"{"type":"minecraft:game_rules","rules":"#.to_vec();
            payload.extend_from_slice(rules);
            payload.push(b'}');
            let mut store = WorldStore::new();
            store.record_registry_entries(
                TEST_ENVIRONMENT_REGISTRY,
                0,
                vec![RegistryPacketEntry::with_known_pack_data_for_test(
                    "example:invalid_rules",
                    payload,
                )],
            );

            let error = store
                .synchronized_test_environment_definition(0)
                .unwrap_err();
            assert!(error.reason.contains(expected), "{error}");
        }

        let mut store = WorldStore::new();
        store.record_registry_entries(
            TEST_ENVIRONMENT_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_known_pack_data_for_test(
                "example:bounded_rules",
                br#"{
                    "type":"minecraft:game_rules",
                    "rules":{
                        "minecraft:fire_spread_radius_around_player":-1,
                        "minecraft:max_minecart_speed":1000,
                        "minecraft:max_snow_accumulation_height":8
                    }
                }"#
                .to_vec(),
            )],
        );
        let definition = store
            .synchronized_test_environment_definition(0)
            .unwrap()
            .unwrap();
        assert!(matches!(
            definition.kind,
            TestEnvironmentKind::GameRules { rules }
                if rules.iter().map(|rule| rule.value).collect::<Vec<_>>()
                    == vec![
                        TestGameRuleTypedValue::Integer(-1),
                        TestGameRuleTypedValue::Integer(1000),
                        TestGameRuleTypedValue::Integer(8),
                    ]
        ));
    }

    fn weather_environment_nbt(weather: &str) -> Vec<u8> {
        let mut out = vec![10, 8];
        write_nbt_string(&mut out, "type");
        write_nbt_string(&mut out, "minecraft:weather");
        out.push(8);
        write_nbt_string(&mut out, "weather");
        write_nbt_string(&mut out, weather);
        out.push(0);
        out
    }

    fn game_rules_environment_nbt(keep_inventory: bool, random_tick_speed: i32) -> Vec<u8> {
        let mut out = vec![10, 8];
        write_nbt_string(&mut out, "type");
        write_nbt_string(&mut out, "minecraft:game_rules");
        out.push(10);
        write_nbt_string(&mut out, "rules");
        out.push(1);
        write_nbt_string(&mut out, "minecraft:keep_inventory");
        out.push(u8::from(keep_inventory));
        out.push(3);
        write_nbt_string(&mut out, "minecraft:random_tick_speed");
        out.extend_from_slice(&random_tick_speed.to_be_bytes());
        out.push(0);
        out.push(0);
        out
    }

    fn write_nbt_string(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }
}
