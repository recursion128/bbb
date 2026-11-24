//! Exact synchronized codecs for the game-test registry family.
//!
//! `test_environment` and `test_instance` use the same DIRECT codec for
//! persistent JSON and synchronized network NBT. Named holders remain
//! placeholders during decode; the public encoder requires frozen generation
//! membership before it emits canonical network NBT

use bbb_facts::game_rules::{vanilla_game_rule, GameRuleValue, GameRuleValueKind};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::{
    holders::{
        decode_file_holder, decode_fixed_holder, encode_file_holder, encode_fixed_holder,
        RegistryHolder,
    },
    synchronized_registry_element_codec, validate_registry_codec_references, RegistryCodecError,
    RegistryCodecReference, RegistryElementData, RegistryElementValueRef, RegistryReferenceLookup,
    RegistryResourceIdentifier,
};

const TEST_ENVIRONMENT_REGISTRY: &str = "minecraft:test_environment";
const TEST_INSTANCE_REGISTRY: &str = "minecraft:test_instance";
const WORLD_CLOCK_REGISTRY: &str = "minecraft:world_clock";
const TIMELINE_REGISTRY: &str = "minecraft:timeline";
const MAX_INLINE_ENVIRONMENT_DEPTH: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameTestRegistryElement(GameTestRegistryElementValue);

impl GameTestRegistryElement {
    pub fn view(&self) -> GameTestElementView<'_> {
        match &self.0 {
            GameTestRegistryElementValue::Environment(value) => {
                GameTestElementView::Environment(value.view())
            }
            GameTestRegistryElementValue::Instance(value) => {
                GameTestElementView::Instance(value.view())
            }
        }
    }
}

pub enum GameTestElementView<'a> {
    Environment(TestEnvironmentView<'a>),
    Instance(TestInstanceView<'a>),
}

pub struct TestEnvironmentView<'a>(&'a TestEnvironmentElement);

impl<'a> TestEnvironmentView<'a> {
    pub fn kind(self) -> TestEnvironmentKindView<'a> {
        match &self.0.kind {
            TestEnvironmentKind::AllOf(values) => {
                TestEnvironmentKindView::AllOf(TestEnvironmentHolders(values))
            }
            TestEnvironmentKind::GameRules(values) => TestEnvironmentKindView::GameRules(values),
            TestEnvironmentKind::ClockTime { clock, time } => {
                TestEnvironmentKindView::ClockTime { clock, time: *time }
            }
            TestEnvironmentKind::Timelines(values) => TestEnvironmentKindView::Timelines(values),
            TestEnvironmentKind::Weather(value) => TestEnvironmentKindView::Weather(*value),
            TestEnvironmentKind::Functions { setup, teardown } => {
                TestEnvironmentKindView::Functions {
                    setup: setup.as_ref(),
                    teardown: teardown.as_ref(),
                }
            }
        }
    }
}

pub enum TestEnvironmentKindView<'a> {
    AllOf(TestEnvironmentHolders<'a>),
    GameRules(&'a [(RegistryResourceIdentifier, GameRuleValue)]),
    ClockTime {
        clock: &'a RegistryResourceIdentifier,
        time: i32,
    },
    Timelines(&'a [RegistryResourceIdentifier]),
    Weather(TestWeather),
    Functions {
        setup: Option<&'a RegistryResourceIdentifier>,
        teardown: Option<&'a RegistryResourceIdentifier>,
    },
}

pub struct TestEnvironmentHolders<'a>(&'a [RegistryHolder<TestEnvironmentElement>]);

impl<'a> TestEnvironmentHolders<'a> {
    pub fn iter(self) -> impl ExactSizeIterator<Item = TestEnvironmentHolderView<'a>> {
        self.0.iter().map(TestEnvironmentHolderView::from)
    }
}

pub enum TestEnvironmentHolderView<'a> {
    Named(&'a RegistryResourceIdentifier),
    Inline(TestEnvironmentView<'a>),
}

impl<'a> From<&'a RegistryHolder<TestEnvironmentElement>> for TestEnvironmentHolderView<'a> {
    fn from(value: &'a RegistryHolder<TestEnvironmentElement>) -> Self {
        match value {
            RegistryHolder::Named(id) => Self::Named(id),
            RegistryHolder::Inline(value) => Self::Inline(value.view()),
        }
    }
}

pub struct TestInstanceView<'a> {
    pub kind: TestInstanceKindView<'a>,
    pub environment: TestEnvironmentHolderView<'a>,
    pub structure: &'a RegistryResourceIdentifier,
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

pub enum TestInstanceKindView<'a> {
    BlockBased,
    Function(&'a RegistryResourceIdentifier),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum GameTestRegistryElementValue {
    Environment(TestEnvironmentElement),
    Instance(TestInstanceElement),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TestEnvironmentElement {
    kind: TestEnvironmentKind,
}

impl TestEnvironmentElement {
    fn view(&self) -> TestEnvironmentView<'_> {
        TestEnvironmentView(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TestEnvironmentKind {
    AllOf(Vec<RegistryHolder<TestEnvironmentElement>>),
    GameRules(Vec<(RegistryResourceIdentifier, GameRuleValue)>),
    ClockTime {
        clock: RegistryResourceIdentifier,
        time: i32,
    },
    Timelines(Vec<RegistryResourceIdentifier>),
    Weather(TestWeather),
    Functions {
        setup: Option<RegistryResourceIdentifier>,
        teardown: Option<RegistryResourceIdentifier>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestWeather {
    Clear,
    Rain,
    Thunder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TestInstanceElement {
    kind: TestInstanceKind,
    environment: RegistryHolder<TestEnvironmentElement>,
    structure: RegistryResourceIdentifier,
    max_ticks: i32,
    setup_ticks: i32,
    required: bool,
    rotation: TestRotation,
    manual_only: bool,
    max_attempts: i32,
    required_successes: i32,
    sky_access: bool,
    padding: i32,
}

impl TestInstanceElement {
    fn view(&self) -> TestInstanceView<'_> {
        TestInstanceView {
            kind: match &self.kind {
                TestInstanceKind::BlockBased => TestInstanceKindView::BlockBased,
                TestInstanceKind::Function(id) => TestInstanceKindView::Function(id),
            },
            environment: TestEnvironmentHolderView::from(&self.environment),
            structure: &self.structure,
            max_ticks: self.max_ticks,
            setup_ticks: self.setup_ticks,
            required: self.required,
            rotation: self.rotation,
            manual_only: self.manual_only,
            max_attempts: self.max_attempts,
            required_successes: self.required_successes,
            sky_access: self.sky_access,
            padding: self.padding,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TestInstanceKind {
    BlockBased,
    Function(RegistryResourceIdentifier),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestRotation {
    None,
    Clockwise90,
    Clockwise180,
    Counterclockwise90,
}

/// Decodes one game-test registry element through its generated source route.
/// Both origins currently name the same DIRECT codec, but route lookup remains
/// explicit so generated-catalog drift is attributed rather than hidden.
pub fn decode_game_test_registry_element(
    registry: &str,
    data: &RegistryElementData,
) -> Result<GameTestRegistryElement, RegistryCodecError> {
    let codec = synchronized_registry_element_codec(registry, data.source)?;
    let value = match registry {
        TEST_ENVIRONMENT_REGISTRY => GameTestRegistryElementValue::Environment(decode_environment(
            data.root(),
            codec,
            "<root>",
            0,
        )?),
        TEST_INSTANCE_REGISTRY => {
            GameTestRegistryElementValue::Instance(decode_instance(data.root(), codec, "<root>")?)
        }
        _ => {
            return Err(RegistryCodecError::coverage(
                registry,
                codec,
                "<registry>",
                "registry is not supported by the game-test codec",
            ))
        }
    };
    Ok(GameTestRegistryElement(value))
}

/// Enumerates every named holder dependency without dereferencing its target.
pub fn game_test_registry_element_references(
    element: &GameTestRegistryElement,
) -> Vec<RegistryCodecReference<'_>> {
    let mut output = Vec::new();
    match &element.0 {
        GameTestRegistryElementValue::Environment(value) => {
            environment_references(value, &mut output)
        }
        GameTestRegistryElementValue::Instance(value) => {
            holder_references(&value.environment, &mut output);
        }
    }
    output
}

/// Encodes canonical synchronized network NBT after proving every placeholder
/// belongs to the frozen generation. This preserves Vanilla's
/// `canSerializeIn(owner)` invariant instead of exposing an unchecked holder
/// encoder.
pub fn encode_game_test_registry_element_network(
    element: &GameTestRegistryElement,
    lookup: &impl RegistryReferenceLookup,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    let (registry, codec) = match element.0 {
        GameTestRegistryElementValue::Environment(_) => (
            TEST_ENVIRONMENT_REGISTRY,
            "TestEnvironmentDefinition.DIRECT_CODEC",
        ),
        GameTestRegistryElementValue::Instance(_) => {
            (TEST_INSTANCE_REGISTRY, "GameTestInstance.DIRECT_CODEC")
        }
    };
    validate_registry_codec_references(
        registry,
        codec,
        game_test_registry_element_references(element),
        lookup,
    )?;
    encode_game_test_registry_element_network_unchecked(element)
}

pub(super) fn encode_game_test_registry_element_network_unchecked(
    element: &GameTestRegistryElement,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    match &element.0 {
        GameTestRegistryElementValue::Environment(value) => encode_environment(value),
        GameTestRegistryElementValue::Instance(value) => encode_instance(value),
    }
}

fn decode_environment(
    value: RegistryElementValueRef<'_>,
    codec: &str,
    path: &str,
    depth: usize,
) -> Result<TestEnvironmentElement, RegistryCodecError> {
    require_compound(value, TEST_ENVIRONMENT_REGISTRY, codec, path)?;
    if depth >= MAX_INLINE_ENVIRONMENT_DEPTH {
        return Err(codec_error(
            TEST_ENVIRONMENT_REGISTRY,
            codec,
            path,
            "inline environment nesting exceeds 64",
        )
        .aborting_speculation());
    }
    let kind = match identifier_field(value, "type", TEST_ENVIRONMENT_REGISTRY, codec, "type")?
        .as_str()
    {
        "minecraft:all_of" => {
            let definitions = required_field(
                value,
                "definitions",
                TEST_ENVIRONMENT_REGISTRY,
                codec,
                "definitions",
            )?;
            TestEnvironmentKind::AllOf(decode_list(
                definitions,
                TEST_ENVIRONMENT_REGISTRY,
                codec,
                "definitions",
                |definition, definition_path| {
                    decode_file_holder(definition_path, definition, |inline, inline_path| {
                        decode_environment(inline, codec, inline_path, depth + 1)
                    })
                },
            )?)
        }
        "minecraft:game_rules" => {
            let rules = required_field(value, "rules", TEST_ENVIRONMENT_REGISTRY, codec, "rules")?;
            let len = rules.map_len().ok_or_else(|| {
                codec_error(
                    TEST_ENVIRONMENT_REGISTRY,
                    codec,
                    "rules",
                    "expected a game-rule map",
                )
            })?;
            let mut decoded = Vec::with_capacity(len);
            for index in 0..len {
                let (raw_id, rule_value) =
                    rules.map_entry_java_string_ref(index).ok_or_else(|| {
                        codec_error(
                            TEST_ENVIRONMENT_REGISTRY,
                            codec,
                            "rules",
                            "game-rule entry is missing",
                        )
                    })?;
                let rule_id =
                    RegistryResourceIdentifier::parse_java_string(raw_id).ok_or_else(|| {
                        codec_error(
                            TEST_ENVIRONMENT_REGISTRY,
                            codec,
                            "rules",
                            "invalid game-rule identifier",
                        )
                    })?;
                let descriptor = vanilla_game_rule(rule_id.as_str()).ok_or_else(|| {
                    codec_error(
                        TEST_ENVIRONMENT_REGISTRY,
                        codec,
                        "rules",
                        "unknown built-in game rule",
                    )
                })?;
                let rule_value = match descriptor.value_kind {
                    GameRuleValueKind::Boolean => rule_value
                        .as_bool()
                        .map(GameRuleValue::Boolean)
                        .ok_or_else(|| {
                            codec_error(
                                TEST_ENVIRONMENT_REGISTRY,
                                codec,
                                "rules",
                                "expected boolean",
                            )
                        })?,
                    GameRuleValueKind::Integer { min, max } => {
                        let value = rule_value.codec_i32().ok_or_else(|| {
                            codec_error(
                                TEST_ENVIRONMENT_REGISTRY,
                                codec,
                                "rules",
                                "expected integer",
                            )
                        })?;
                        if !(min..=max).contains(&value) {
                            return Err(codec_error(
                                TEST_ENVIRONMENT_REGISTRY,
                                codec,
                                "rules",
                                format!("integer outside {min}..={max}"),
                            ));
                        }
                        GameRuleValue::Integer(value)
                    }
                };
                decoded.push((rule_id, rule_value));
            }
            decoded.sort_by(|left, right| left.0.as_str().cmp(right.0.as_str()));
            TestEnvironmentKind::GameRules(decoded)
        }
        "minecraft:clock_time" => {
            let clock = decode_fixed_holder(
                TEST_ENVIRONMENT_REGISTRY,
                codec,
                "clock",
                required_field(value, "clock", TEST_ENVIRONMENT_REGISTRY, codec, "clock")?,
            )?;
            let time = int_field(value, "time", TEST_ENVIRONMENT_REGISTRY, codec, "time")?;
            if time < 0 {
                return Err(codec_error(
                    TEST_ENVIRONMENT_REGISTRY,
                    codec,
                    "time",
                    "expected non-negative integer",
                ));
            }
            TestEnvironmentKind::ClockTime { clock, time }
        }
        "minecraft:timeline_attributes" => {
            let timelines = required_field(
                value,
                "timelines",
                TEST_ENVIRONMENT_REGISTRY,
                codec,
                "timelines",
            )?;
            TestEnvironmentKind::Timelines(decode_list(
                timelines,
                TEST_ENVIRONMENT_REGISTRY,
                codec,
                "timelines",
                |timeline, timeline_path| {
                    decode_fixed_holder(TEST_ENVIRONMENT_REGISTRY, codec, timeline_path, timeline)
                },
            )?)
        }
        "minecraft:weather" => {
            let weather = match string_field(
                value,
                "weather",
                TEST_ENVIRONMENT_REGISTRY,
                codec,
                "weather",
            )? {
                value if value == "clear" => TestWeather::Clear,
                value if value == "rain" => TestWeather::Rain,
                value if value == "thunder" => TestWeather::Thunder,
                _ => {
                    return Err(codec_error(
                        TEST_ENVIRONMENT_REGISTRY,
                        codec,
                        "weather",
                        "unknown weather",
                    ))
                }
            };
            TestEnvironmentKind::Weather(weather)
        }
        "minecraft:function" => TestEnvironmentKind::Functions {
            setup: optional_identifier_field(
                value,
                "setup",
                TEST_ENVIRONMENT_REGISTRY,
                codec,
                "setup",
            )?,
            teardown: optional_identifier_field(
                value,
                "teardown",
                TEST_ENVIRONMENT_REGISTRY,
                codec,
                "teardown",
            )?,
        },
        _ => {
            return Err(codec_error(
                TEST_ENVIRONMENT_REGISTRY,
                codec,
                "type",
                "unknown test-environment dispatch id",
            ))
        }
    };
    Ok(TestEnvironmentElement { kind })
}

fn decode_instance(
    value: RegistryElementValueRef<'_>,
    codec: &str,
    path: &str,
) -> Result<TestInstanceElement, RegistryCodecError> {
    require_compound(value, TEST_INSTANCE_REGISTRY, codec, path)?;
    let kind =
        match identifier_field(value, "type", TEST_INSTANCE_REGISTRY, codec, "type")?.as_str() {
            "minecraft:block_based" => TestInstanceKind::BlockBased,
            "minecraft:function" => TestInstanceKind::Function(identifier_field(
                value,
                "function",
                TEST_INSTANCE_REGISTRY,
                codec,
                "function",
            )?),
            _ => {
                return Err(codec_error(
                    TEST_INSTANCE_REGISTRY,
                    codec,
                    "type",
                    "unknown test-instance dispatch id",
                ))
            }
        };

    let environment = decode_file_holder(
        "environment",
        required_field(
            value,
            "environment",
            TEST_INSTANCE_REGISTRY,
            codec,
            "environment",
        )?,
        |inline, inline_path| decode_environment(inline, codec, inline_path, 0),
    )?;
    let structure = identifier_field(
        value,
        "structure",
        TEST_INSTANCE_REGISTRY,
        codec,
        "structure",
    )?;
    let max_ticks = int_field(
        value,
        "max_ticks",
        TEST_INSTANCE_REGISTRY,
        codec,
        "max_ticks",
    )?;
    if max_ticks <= 0 {
        return Err(codec_error(
            TEST_INSTANCE_REGISTRY,
            codec,
            "max_ticks",
            "expected positive integer",
        ));
    }
    let setup_ticks = optional_bounded_int(value, "setup_ticks", 0, i32::MAX, 0, codec)?;
    let required = optional_bool(value, "required", true, codec)?;
    let rotation = match optional_string(value, "rotation", codec)? {
        None => TestRotation::None,
        Some(value) if value == "none" => TestRotation::None,
        Some(value) if value == "clockwise_90" => TestRotation::Clockwise90,
        Some(value) if value == "180" => TestRotation::Clockwise180,
        Some(value) if value == "counterclockwise_90" => TestRotation::Counterclockwise90,
        _ => {
            return Err(codec_error(
                TEST_INSTANCE_REGISTRY,
                codec,
                "rotation",
                "unknown rotation",
            ))
        }
    };
    let manual_only = optional_bool(value, "manual_only", false, codec)?;
    let max_attempts = optional_bounded_int(value, "max_attempts", 1, i32::MAX, 1, codec)?;
    let required_successes =
        optional_bounded_int(value, "required_successes", 1, i32::MAX, 1, codec)?;
    let sky_access = optional_bool(value, "sky_access", false, codec)?;
    let padding = optional_bounded_int(value, "padding", 0, 128, 0, codec)?;

    Ok(TestInstanceElement {
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

fn encode_environment(
    value: &TestEnvironmentElement,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    let mut fields = Vec::new();
    match &value.kind {
        TestEnvironmentKind::AllOf(definitions) => {
            fields.push(entry("type", string("minecraft:all_of")));
            fields.push(entry(
                "definitions",
                NbtSummaryValue::List(
                    definitions
                        .iter()
                        .map(|holder| encode_file_holder(holder, encode_environment))
                        .collect::<Result<Vec<_>, _>>()?,
                ),
            ));
        }
        TestEnvironmentKind::GameRules(rules) => {
            fields.push(entry("type", string("minecraft:game_rules")));
            fields.push(entry(
                "rules",
                NbtSummaryValue::Compound(
                    rules
                        .iter()
                        .map(|(resource_id, value)| {
                            entry(
                                resource_id.as_str(),
                                match value {
                                    GameRuleValue::Boolean(value) => {
                                        NbtSummaryValue::Byte(i8::from(*value))
                                    }
                                    GameRuleValue::Integer(value) => NbtSummaryValue::Int(*value),
                                },
                            )
                        })
                        .collect(),
                ),
            ));
        }
        TestEnvironmentKind::ClockTime { clock, time } => {
            fields.push(entry("type", string("minecraft:clock_time")));
            fields.push(entry("clock", encode_fixed_holder(clock)));
            fields.push(entry("time", NbtSummaryValue::Int(*time)));
        }
        TestEnvironmentKind::Timelines(timelines) => {
            fields.push(entry("type", string("minecraft:timeline_attributes")));
            fields.push(entry(
                "timelines",
                NbtSummaryValue::List(timelines.iter().map(encode_fixed_holder).collect()),
            ));
        }
        TestEnvironmentKind::Weather(weather) => {
            fields.push(entry("type", string("minecraft:weather")));
            fields.push(entry(
                "weather",
                string(match weather {
                    TestWeather::Clear => "clear",
                    TestWeather::Rain => "rain",
                    TestWeather::Thunder => "thunder",
                }),
            ));
        }
        TestEnvironmentKind::Functions { setup, teardown } => {
            fields.push(entry("type", string("minecraft:function")));
            if let Some(setup) = setup {
                fields.push(entry("setup", encode_fixed_holder(setup)));
            }
            if let Some(teardown) = teardown {
                fields.push(entry("teardown", encode_fixed_holder(teardown)));
            }
        }
    }
    Ok(NbtSummaryValue::Compound(fields))
}

fn encode_instance(value: &TestInstanceElement) -> Result<NbtSummaryValue, RegistryCodecError> {
    let mut fields = Vec::new();
    match &value.kind {
        TestInstanceKind::BlockBased => fields.push(entry("type", string("minecraft:block_based"))),
        TestInstanceKind::Function(function) => {
            fields.push(entry("type", string("minecraft:function")));
            fields.push(entry("function", encode_fixed_holder(function)));
        }
    }
    fields.push(entry(
        "environment",
        encode_file_holder(&value.environment, encode_environment)?,
    ));
    fields.push(entry("structure", encode_fixed_holder(&value.structure)));
    fields.push(entry("max_ticks", NbtSummaryValue::Int(value.max_ticks)));
    if value.setup_ticks != 0 {
        fields.push(entry(
            "setup_ticks",
            NbtSummaryValue::Int(value.setup_ticks),
        ));
    }
    if !value.required {
        fields.push(entry("required", NbtSummaryValue::Byte(0)));
    }
    if value.rotation != TestRotation::None {
        fields.push(entry(
            "rotation",
            string(match value.rotation {
                TestRotation::None => unreachable!(),
                TestRotation::Clockwise90 => "clockwise_90",
                TestRotation::Clockwise180 => "180",
                TestRotation::Counterclockwise90 => "counterclockwise_90",
            }),
        ));
    }
    if value.manual_only {
        fields.push(entry("manual_only", NbtSummaryValue::Byte(1)));
    }
    if value.max_attempts != 1 {
        fields.push(entry(
            "max_attempts",
            NbtSummaryValue::Int(value.max_attempts),
        ));
    }
    if value.required_successes != 1 {
        fields.push(entry(
            "required_successes",
            NbtSummaryValue::Int(value.required_successes),
        ));
    }
    if value.sky_access {
        fields.push(entry("sky_access", NbtSummaryValue::Byte(1)));
    }
    if value.padding != 0 {
        fields.push(entry("padding", NbtSummaryValue::Int(value.padding)));
    }
    Ok(NbtSummaryValue::Compound(fields))
}

fn environment_references<'a>(
    value: &'a TestEnvironmentElement,
    output: &mut Vec<RegistryCodecReference<'a>>,
) {
    match &value.kind {
        TestEnvironmentKind::AllOf(definitions) => {
            for holder in definitions {
                holder_references(holder, output);
            }
        }
        TestEnvironmentKind::ClockTime { clock, .. } => output.push(RegistryCodecReference {
            target_registry: WORLD_CLOCK_REGISTRY,
            kind: super::RegistryReferenceKind::Element,
            resource_id: clock,
        }),
        TestEnvironmentKind::Timelines(timelines) => {
            output.extend(timelines.iter().map(|resource_id| RegistryCodecReference {
                target_registry: TIMELINE_REGISTRY,
                kind: super::RegistryReferenceKind::Element,
                resource_id,
            }));
        }
        TestEnvironmentKind::GameRules(_)
        | TestEnvironmentKind::Weather(_)
        | TestEnvironmentKind::Functions { .. } => {}
    }
}

fn holder_references<'a>(
    holder: &'a RegistryHolder<TestEnvironmentElement>,
    output: &mut Vec<RegistryCodecReference<'a>>,
) {
    if let Some(reference) = holder.named_reference(TEST_ENVIRONMENT_REGISTRY) {
        output.push(reference);
    } else if let RegistryHolder::Inline(value) = holder {
        environment_references(value, output);
    }
}

fn decode_list<T>(
    value: RegistryElementValueRef<'_>,
    registry: &str,
    codec: &str,
    path: &str,
    mut decode: impl FnMut(RegistryElementValueRef<'_>, &str) -> Result<T, RegistryCodecError>,
) -> Result<Vec<T>, RegistryCodecError> {
    let len = value
        .list_len()
        .ok_or_else(|| codec_error(registry, codec, path, "expected a list"))?;
    let mut output = Vec::with_capacity(len);
    for index in 0..len {
        let item = value
            .list_element(index)
            .ok_or_else(|| codec_error(registry, codec, path, "list element is missing"))?;
        output.push(decode(item, path)?);
    }
    Ok(output)
}

fn require_compound(
    value: RegistryElementValueRef<'_>,
    registry: &str,
    codec: &str,
    path: &str,
) -> Result<(), RegistryCodecError> {
    if value.is_compound() {
        Ok(())
    } else {
        Err(codec_error(registry, codec, path, "expected a compound"))
    }
}

fn required_field<'a>(
    value: RegistryElementValueRef<'a>,
    name: &str,
    registry: &str,
    codec: &str,
    path: &str,
) -> Result<RegistryElementValueRef<'a>, RegistryCodecError> {
    value
        .field(name)
        .ok_or_else(|| codec_error(registry, codec, path, "required field is missing"))
}

fn string_field<'a>(
    value: RegistryElementValueRef<'a>,
    name: &str,
    registry: &str,
    codec: &str,
    path: &str,
) -> Result<&'a bbb_protocol::JavaString, RegistryCodecError> {
    required_field(value, name, registry, codec, path)?
        .java_string_ref()
        .ok_or_else(|| codec_error(registry, codec, path, "expected a string"))
}

fn identifier_field(
    value: RegistryElementValueRef<'_>,
    name: &str,
    registry: &str,
    codec: &str,
    path: &str,
) -> Result<RegistryResourceIdentifier, RegistryCodecError> {
    let raw = string_field(value, name, registry, codec, path)?;
    RegistryResourceIdentifier::parse_java_string(raw)
        .ok_or_else(|| codec_error(registry, codec, path, "invalid resource identifier"))
}

fn optional_identifier_field(
    value: RegistryElementValueRef<'_>,
    name: &str,
    registry: &str,
    codec: &str,
    path: &str,
) -> Result<Option<RegistryResourceIdentifier>, RegistryCodecError> {
    value
        .field(name)
        .map(|field| {
            field
                .java_string_ref()
                .and_then(RegistryResourceIdentifier::parse_java_string)
                .ok_or_else(|| codec_error(registry, codec, path, "invalid resource identifier"))
        })
        .transpose()
}

fn int_field(
    value: RegistryElementValueRef<'_>,
    name: &str,
    registry: &str,
    codec: &str,
    path: &str,
) -> Result<i32, RegistryCodecError> {
    required_field(value, name, registry, codec, path)?
        .codec_i32()
        .ok_or_else(|| codec_error(registry, codec, path, "expected an integer"))
}

fn optional_bounded_int(
    value: RegistryElementValueRef<'_>,
    name: &str,
    min: i32,
    max: i32,
    default: i32,
    codec: &str,
) -> Result<i32, RegistryCodecError> {
    let Some(field) = value.field(name) else {
        return Ok(default);
    };
    let decoded = field
        .codec_i32()
        .ok_or_else(|| codec_error(TEST_INSTANCE_REGISTRY, codec, name, "expected an integer"))?;
    if (min..=max).contains(&decoded) {
        Ok(decoded)
    } else {
        Err(codec_error(
            TEST_INSTANCE_REGISTRY,
            codec,
            name,
            format!("integer outside {min}..={max}"),
        ))
    }
}

fn optional_bool(
    value: RegistryElementValueRef<'_>,
    name: &str,
    default: bool,
    codec: &str,
) -> Result<bool, RegistryCodecError> {
    value
        .field(name)
        .map(|field| {
            field.as_bool().ok_or_else(|| {
                codec_error(TEST_INSTANCE_REGISTRY, codec, name, "expected a boolean")
            })
        })
        .transpose()
        .map(|value| value.unwrap_or(default))
}

fn optional_string<'a>(
    value: RegistryElementValueRef<'a>,
    name: &str,
    codec: &str,
) -> Result<Option<&'a bbb_protocol::JavaString>, RegistryCodecError> {
    value
        .field(name)
        .map(|field| {
            field.java_string_ref().ok_or_else(|| {
                codec_error(TEST_INSTANCE_REGISTRY, codec, name, "expected a string")
            })
        })
        .transpose()
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn string(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::String(value.into())
}

fn codec_error(
    registry: &str,
    codec: &str,
    field: impl Into<String>,
    reason: impl Into<String>,
) -> RegistryCodecError {
    RegistryCodecError {
        registry: registry.to_string(),
        codec: codec.to_string(),
        field: field.into(),
        reason: reason.into(),
        abort_speculation: false,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::registry_codec::{decode_direct_registry_json, RegistryElementSource};

    struct Lookup {
        elements: BTreeSet<(&'static str, String)>,
    }

    impl RegistryReferenceLookup for Lookup {
        fn contains_element(&self, registry: &str, resource_id: &str) -> bool {
            self.elements
                .contains(&(registry_static(registry), resource_id.to_string()))
        }

        fn contains_tag(&self, _registry: &str, _resource_id: &str) -> bool {
            false
        }
    }

    fn registry_static(registry: &str) -> &'static str {
        match registry {
            TEST_ENVIRONMENT_REGISTRY => TEST_ENVIRONMENT_REGISTRY,
            WORLD_CLOCK_REGISTRY => WORLD_CLOCK_REGISTRY,
            TIMELINE_REGISTRY => TIMELINE_REGISTRY,
            _ => panic!("unexpected registry {registry}"),
        }
    }

    fn lookup(entries: &[(&'static str, &str)]) -> Lookup {
        Lookup {
            elements: entries
                .iter()
                .map(|(registry, id)| (*registry, (*id).to_string()))
                .collect(),
        }
    }

    fn decode(registry: &str, json: &[u8]) -> GameTestRegistryElement {
        let data = decode_direct_registry_json(json).unwrap();
        assert_eq!(data.source, RegistryElementSource::ServerDatapackJson);
        decode_game_test_registry_element(registry, &data).unwrap()
    }

    #[test]
    fn all_environment_dispatches_encode_canonical_network_shapes() {
        let cases = [
            (
                br#"{"type":"all_of","definitions":["example:self",{"type":"weather","weather":"rain"}]}"#.as_slice(),
                lookup(&[(TEST_ENVIRONMENT_REGISTRY, "example:self")]),
            ),
            (
                br#"{"type":"game_rules","rules":{"random_tick_speed":3.9,"keep_inventory":true}}"#.as_slice(),
                lookup(&[]),
            ),
            (
                br#"{"type":"clock_time","clock":"example:clock","time":6000.9}"#.as_slice(),
                lookup(&[(WORLD_CLOCK_REGISTRY, "example:clock")]),
            ),
            (
                br#"{"type":"timeline_attributes","timelines":["example:timeline"]}"#.as_slice(),
                lookup(&[(TIMELINE_REGISTRY, "example:timeline")]),
            ),
            (
                br#"{"type":"weather","weather":"thunder"}"#.as_slice(),
                lookup(&[]),
            ),
            (
                br#"{"type":"function","setup":"example:setup","teardown":null}"#.as_slice(),
                lookup(&[]),
            ),
        ];

        for (json, lookup) in cases {
            let element = decode(TEST_ENVIRONMENT_REGISTRY, json);
            let encoded = encode_game_test_registry_element_network(&element, &lookup).unwrap();
            let network = RegistryElementData::from_network_nbt(encoded);
            assert_eq!(
                decode_game_test_registry_element(TEST_ENVIRONMENT_REGISTRY, &network).unwrap(),
                element
            );
        }
    }

    #[test]
    fn instance_defaults_and_non_defaults_use_exact_nbt_tags() {
        let defaults = decode(
            TEST_INSTANCE_REGISTRY,
            br#"{
                "type":"block_based",
                "environment":{"type":"weather","weather":"clear"},
                "structure":"example:room",
                "max_ticks":20
            }"#,
        );
        let encoded = encode_game_test_registry_element_network(&defaults, &lookup(&[])).unwrap();
        let NbtSummaryValue::Compound(fields) = encoded else {
            panic!("expected compound")
        };
        assert_eq!(
            fields.iter().map(|field| &field.name).collect::<Vec<_>>(),
            ["type", "environment", "structure", "max_ticks"]
        );
        assert!(matches!(fields[3].value, NbtSummaryValue::Int(20)));

        let non_defaults = decode(
            TEST_INSTANCE_REGISTRY,
            br#"{
                "type":"function",
                "function":"example:test",
                "environment":"example:batch",
                "structure":"example:room",
                "max_ticks":40.9,
                "setup_ticks":1,
                "required":false,
                "rotation":"clockwise_90",
                "manual_only":true,
                "max_attempts":3,
                "required_successes":2,
                "sky_access":true,
                "padding":16
            }"#,
        );
        let encoded = encode_game_test_registry_element_network(
            &non_defaults,
            &lookup(&[(TEST_ENVIRONMENT_REGISTRY, "example:batch")]),
        )
        .unwrap();
        let NbtSummaryValue::Compound(fields) = encoded else {
            panic!("expected compound")
        };
        assert!(fields
            .iter()
            .any(|field| field.name == "required" && field.value == NbtSummaryValue::Byte(0)));
        assert!(fields
            .iter()
            .any(|field| field.name == "padding" && field.value == NbtSummaryValue::Int(16)));
    }

    #[test]
    fn network_nbt_uses_codec_numeric_narrowing_and_numeric_booleans() {
        let data = RegistryElementData::from_network_nbt(NbtSummaryValue::Compound(vec![
            entry("type", string("minecraft:game_rules")),
            entry(
                "rules",
                NbtSummaryValue::Compound(vec![
                    entry("minecraft:keep_inventory", NbtSummaryValue::Int(2)),
                    entry(
                        "minecraft:random_tick_speed",
                        NbtSummaryValue::Double(7.9_f64.to_bits()),
                    ),
                ]),
            ),
        ]));
        let element = decode_game_test_registry_element(TEST_ENVIRONMENT_REGISTRY, &data).unwrap();
        let encoded = encode_game_test_registry_element_network(&element, &lookup(&[])).unwrap();
        assert!(matches!(
            encoded,
            NbtSummaryValue::Compound(fields)
                if matches!(
                    fields.iter().find(|field| field.name == "rules").map(|field| &field.value),
                    Some(NbtSummaryValue::Compound(rules))
                        if rules.iter().any(|rule| rule.name == "minecraft:keep_inventory"
                            && rule.value == NbtSummaryValue::Byte(1))
                            && rules.iter().any(|rule| rule.name == "minecraft:random_tick_speed"
                                && rule.value == NbtSummaryValue::Int(7))
                )
        ));
    }
}
