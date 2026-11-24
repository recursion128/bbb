//! Exact synchronized `DimensionType` owner.
//!
//! Vanilla decodes server datapack JSON through `DIRECT_CODEC`, then encodes
//! synchronized publication through `NETWORK_CODEC`. Client known-pack and
//! network values already use `NETWORK_CODEC`. This owner always emits that
//! canonical network shape while retaining every resolution-bearing holder
//! and holder-set tag dependency until the frozen registry generation
//! validates membership. Plain `TagKey` values such as `infiniburn` only
//! validate their `#id` syntax.

use crate::persistent_component_nbt::source_codec::decode_homogeneous_fixed_holder_set_source;
use crate::registry_codec::{
    recursive_values::{required_field, OwnedRegistryCodecReference, RegistryCodecDecodeContext},
    synchronized_registry_element_codec, RegistryCodecError, RegistryElementData,
    RegistryElementValueRef, RegistryResourceIdentifier,
};
use bbb_protocol::{
    nbt_summary::{canonical_double_tag_bits, canonical_float_tag_bits, entry},
    NbtSummaryValue,
};

use super::environment_source::{
    decode_environment_attribute_map_source, EnvironmentAttributeMapMode,
};
use super::value_provider_source::{decode_int_provider_in, IntProvider};

const DIMENSION_TYPE_REGISTRY: &str = "minecraft:dimension_type";
const TIMELINE_REGISTRY: &str = "minecraft:timeline";
const WORLD_CLOCK_REGISTRY: &str = "minecraft:world_clock";

const LIGHT_LEVEL_MIN: i32 = 0;
const LIGHT_LEVEL_MAX: i32 = 15;
const COORDINATE_SCALE_MIN: f64 = 1.0e-5_f32 as f64;
const COORDINATE_SCALE_MAX: f64 = 3.0e7_f64;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DimensionTypeScalars {
    pub has_fixed_time: bool,
    pub has_skylight: bool,
    pub has_ceiling: bool,
    pub has_ender_dragon_fight: bool,
    pub coordinate_scale: f64,
    pub min_y: i32,
    pub height: i32,
    pub logical_height: i32,
    pub ambient_light: f32,
    pub monster_spawn_block_light_limit: i32,
    pub skybox: DimensionSkybox,
    pub cardinal_light: DimensionCardinalLight,
}

impl DimensionTypeScalars {
    // DimensionType.java:48-52; BlockPos.java:49-50.
    pub const Y_SIZE: i32 =
        (1 << (64 - 2 * (1 + 30_000_000_u32.next_power_of_two().trailing_zeros()))) - 32;
    pub const MAX_Y: i32 = (Self::Y_SIZE >> 1) - 1;
    pub const MIN_Y: i32 = Self::MAX_Y - Self::Y_SIZE + 1;

    /// DimensionType. Generators only know the physical range.
    #[inline]
    pub fn check_shape(
        min_y: i32,
        height: i32,
        logical_height: Option<i32>,
    ) -> Result<(), &'static str> {
        if !(Self::MIN_Y..=Self::MAX_Y).contains(&min_y) {
            return Err("min_y is outside dimension bounds");
        }
        if !(16..=Self::Y_SIZE).contains(&height) {
            return Err("height is outside dimension bounds");
        }
        if logical_height.is_some_and(|value| !(0..=Self::Y_SIZE).contains(&value)) {
            return Err("logical_height is outside dimension bounds");
        }
        if min_y + height > Self::MAX_Y + 1 {
            return Err("min_y + height exceeds dimension maximum");
        }
        if logical_height.is_some_and(|logical_height| logical_height > height) {
            return Err("logical_height cannot be higher than height");
        }
        if height % 16 != 0 {
            return Err("height has to be multiple of 16");
        }
        if min_y % 16 != 0 {
            return Err("min_y has to be a multiple of 16");
        }
        Ok(())
    }
}

/// Runtime facts from a complete dimension-type decode. The tag excludes `#`.
#[derive(Debug, Clone, PartialEq)]
pub struct DirectDimensionTypeFacts {
    pub scalars: DimensionTypeScalars,
    pub infiniburn_tag: String,
    pub monster_spawn_light_level: IntProvider,
    pub default_clock: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecodedDimensionType {
    pub facts: DirectDimensionTypeFacts,
    pub attributes: Option<NbtSummaryValue>,
    pub timelines: Option<NbtSummaryValue>,
    pub(crate) references: Vec<OwnedRegistryCodecReference>,
}

impl DecodedDimensionType {
    pub(crate) fn projected_network_nbt(&self) -> NbtSummaryValue {
        encode_dimension_type(
            &self.facts.scalars,
            &self.facts.infiniburn_tag,
            self.facts.default_clock.as_deref(),
            self.facts.monster_spawn_light_level.network_nbt(),
            self.attributes.clone(),
            self.timelines.clone(),
        )
    }

    fn into_network_nbt(self) -> NbtSummaryValue {
        encode_dimension_type(
            &self.facts.scalars,
            &self.facts.infiniburn_tag,
            self.facts.default_clock.as_deref(),
            self.facts.monster_spawn_light_level.into_nbt(),
            self.attributes,
            self.timelines,
        )
    }
}

fn encode_dimension_type(
    facts: &DimensionTypeScalars,
    infiniburn_tag: &str,
    default_clock: Option<&str>,
    monster_spawn_light_level: NbtSummaryValue,
    attributes: Option<NbtSummaryValue>,
    timelines: Option<NbtSummaryValue>,
) -> NbtSummaryValue {
    let mut output = Vec::with_capacity(17);
    if facts.has_fixed_time {
        output.push(entry("has_fixed_time", NbtSummaryValue::Byte(1)));
    }
    for (name, value) in [
        ("has_skylight", facts.has_skylight),
        ("has_ceiling", facts.has_ceiling),
        ("has_ender_dragon_fight", facts.has_ender_dragon_fight),
    ] {
        output.push(entry(name, NbtSummaryValue::Byte(i8::from(value))));
    }
    output.push(entry(
        "coordinate_scale",
        NbtSummaryValue::Double(canonical_double_tag_bits(facts.coordinate_scale)),
    ));
    output.push(entry("min_y", NbtSummaryValue::Int(facts.min_y)));
    output.push(entry("height", NbtSummaryValue::Int(facts.height)));
    output.push(entry(
        "logical_height",
        NbtSummaryValue::Int(facts.logical_height),
    ));
    output.push(entry(
        "infiniburn",
        NbtSummaryValue::String(format!("#{infiniburn_tag}").into()),
    ));
    output.push(entry(
        "ambient_light",
        NbtSummaryValue::Float(canonical_float_tag_bits(facts.ambient_light)),
    ));
    output.push(entry(
        "monster_spawn_light_level",
        monster_spawn_light_level,
    ));
    output.push(entry(
        "monster_spawn_block_light_limit",
        NbtSummaryValue::Int(facts.monster_spawn_block_light_limit),
    ));
    if facts.skybox != DimensionSkybox::Overworld {
        output.push(entry(
            "skybox",
            NbtSummaryValue::String(facts.skybox.serialized_name().into()),
        ));
    }
    if facts.cardinal_light != DimensionCardinalLight::Default {
        output.push(entry(
            "cardinal_light",
            NbtSummaryValue::String(facts.cardinal_light.serialized_name().into()),
        ));
    }
    if let Some(attributes) = attributes {
        output.push(entry("attributes", attributes));
    }
    if let Some(timelines) = timelines {
        output.push(entry("timelines", timelines));
    }
    if let Some(clock) = default_clock {
        output.push(entry(
            "default_clock",
            NbtSummaryValue::String(clock.into()),
        ));
    }
    NbtSummaryValue::Compound(output)
}

/// Canonical persistent `DimensionType.DIRECT_CODEC` value for equality checks.
///
/// Direct and network dimension types differ only in their environment-attribute
/// codec, so this path
/// deliberately retains every direct attribute before recursively sorting map keys.
pub fn canonical_direct_dimension_type(
    data: &RegistryElementData,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    canonical_direct_dimension_type_with_content_registry(
        data,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

pub fn canonical_direct_dimension_type_with_content_registry(
    data: &RegistryElementData,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    let decoded = decode_direct_dimension_type_with_content_registry(data, content_registry)?;
    let mut canonical = decoded.into_network_nbt();
    canonicalize_map_order(&mut canonical);
    Ok(canonical)
}

/// Decode all `DimensionType.DIRECT_CODEC` fields and retain its runtime facts.
///
/// `coordinate_scale` is the teleportation scale input.
pub fn decode_direct_dimension_type_facts(
    data: &RegistryElementData,
) -> Result<DirectDimensionTypeFacts, RegistryCodecError> {
    decode_direct_dimension_type_facts_with_content_registry(
        data,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

pub fn decode_direct_dimension_type_facts_with_content_registry(
    data: &RegistryElementData,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<DirectDimensionTypeFacts, RegistryCodecError> {
    decode_direct_dimension_type_with_content_registry(data, content_registry)
        .map(|decoded| decoded.facts)
}

fn decode_direct_dimension_type_with_content_registry(
    data: &RegistryElementData,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<DecodedDimensionType, RegistryCodecError> {
    let codec = synchronized_registry_element_codec(DIMENSION_TYPE_REGISTRY, data.source)?;
    let mut context = RegistryCodecDecodeContext::new(DIMENSION_TYPE_REGISTRY, codec, data.source)
        .with_content_registry(content_registry);
    let decoded = decode_dimension_type_source(
        data.root(),
        "$",
        &mut context,
        EnvironmentAttributeMapMode::Direct,
    )?;
    Ok(decoded)
}

fn canonicalize_map_order(value: &mut NbtSummaryValue) {
    match value {
        NbtSummaryValue::Compound(entries) => {
            for entry in entries.iter_mut() {
                canonicalize_map_order(&mut entry.value);
            }
            entries.sort_by(|left, right| left.name.cmp(&right.name));
        }
        NbtSummaryValue::List(values) => {
            for value in values {
                canonicalize_map_order(value);
            }
        }
        _ => {}
    }
}

pub(crate) fn decode_dimension_type_element_with_content_registry(
    data: &RegistryElementData,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<DecodedDimensionType, RegistryCodecError> {
    let codec = synchronized_registry_element_codec(DIMENSION_TYPE_REGISTRY, data.source)?;
    let mut context = RegistryCodecDecodeContext::new(DIMENSION_TYPE_REGISTRY, codec, data.source)
        .with_content_registry(content_registry);
    decode_dimension_type_source(
        data.root(),
        "$",
        &mut context,
        EnvironmentAttributeMapMode::Network,
    )
}

#[cfg(test)]
pub(crate) fn decode_dimension_type_element(
    data: &RegistryElementData,
) -> Result<DecodedDimensionType, RegistryCodecError> {
    decode_dimension_type_element_with_content_registry(
        data,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

fn decode_dimension_type_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    attributes_mode: EnvironmentAttributeMapMode,
) -> Result<DecodedDimensionType, RegistryCodecError> {
    context.charge_record(input, field, "DimensionType record")?;
    let has_fixed_time = input
        .field("has_fixed_time")
        .map(|value| context.decode_bool(value, "has_fixed_time"))
        .transpose()?
        .unwrap_or(false);
    let has_skylight = required_bool_field(input, "has_skylight", field, context)?;
    let has_ceiling = required_bool_field(input, "has_ceiling", field, context)?;
    let has_ender_dragon_fight =
        required_bool_field(input, "has_ender_dragon_fight", field, context)?;
    let coordinate_scale = context.decode_f64(
        required_field(input, "coordinate_scale", field, context)?,
        "coordinate_scale",
    )?;
    if java_double_compare(coordinate_scale, COORDINATE_SCALE_MIN).is_lt()
        || java_double_compare(coordinate_scale, COORDINATE_SCALE_MAX).is_gt()
    {
        return Err(context.codec_error(
            "coordinate_scale",
            format!(
                "double is outside {}..={}",
                COORDINATE_SCALE_MIN, COORDINATE_SCALE_MAX
            ),
        ));
    }
    let min_y = context.decode_i32(required_field(input, "min_y", field, context)?, "min_y")?;
    let height = context.decode_i32(required_field(input, "height", field, context)?, "height")?;
    let logical_height = context.decode_i32(
        required_field(input, "logical_height", field, context)?,
        "logical_height",
    )?;
    DimensionTypeScalars::check_shape(min_y, height, Some(logical_height))
        .map_err(|reason| context.codec_error(field, reason))?;
    let infiniburn_tag = decode_tag_identifier(
        required_field(input, "infiniburn", field, context)?,
        "infiniburn",
        context,
    )?;
    let ambient_light = context.decode_f32(
        required_field(input, "ambient_light", field, context)?,
        "ambient_light",
    )?;
    let ambient_light = f32::from_bits(canonical_float_tag_bits(ambient_light));
    let monster_spawn_light_level =
        context.with_nested("monster_spawn_light_level", |context| {
            decode_int_provider_in(
                required_field(input, "monster_spawn_light_level", field, context)?,
                "monster_spawn_light_level",
                context,
                LIGHT_LEVEL_MIN,
                LIGHT_LEVEL_MAX,
            )
        })?;
    let monster_spawn_block_light_limit = context.decode_i32_range(
        required_field(input, "monster_spawn_block_light_limit", field, context)?,
        "monster_spawn_block_light_limit",
        LIGHT_LEVEL_MIN,
        LIGHT_LEVEL_MAX,
    )?;
    let skybox =
        optional_skybox_field(input, "skybox", context)?.unwrap_or(DimensionSkybox::Overworld);
    let cardinal_light = optional_cardinal_light_field(input, "cardinal_light", context)?
        .unwrap_or(DimensionCardinalLight::Default);

    let mut references = Vec::new();
    let attributes = if let Some(input) = input.field("attributes") {
        let value = context.with_nested("attributes", |context| {
            decode_environment_attribute_map_source(input, "attributes", context, attributes_mode)
        })?;
        let (nbt, attribute_refs) = value.into_parts();
        references.extend(attribute_refs);
        (!is_empty_compound(&nbt)).then_some(nbt)
    } else {
        None
    };
    let timelines = if let Some(input) = input.field("timelines") {
        let value = context.with_nested("timelines", |context| {
            decode_homogeneous_fixed_holder_set_source(
                input,
                "timelines",
                context,
                TIMELINE_REGISTRY,
            )
        })?;
        let (nbt, timeline_refs) = value.into_parts();
        references.extend(timeline_refs);
        (!matches!(&nbt, NbtSummaryValue::List(values) if values.is_empty())).then_some(nbt)
    } else {
        None
    };
    let default_clock = if let Some(input) = input.field("default_clock") {
        context.charge_node("default_clock")?;
        let identifier = parse_identifier(input, "default_clock", context)?;
        let clock = identifier.as_str().to_owned();
        references.push(OwnedRegistryCodecReference::element(
            WORLD_CLOCK_REGISTRY,
            identifier,
        ));
        Some(clock)
    } else {
        None
    };
    Ok(DecodedDimensionType {
        facts: DirectDimensionTypeFacts {
            scalars: DimensionTypeScalars {
                has_fixed_time,
                has_skylight,
                has_ceiling,
                has_ender_dragon_fight,
                coordinate_scale,
                min_y,
                height,
                logical_height,
                ambient_light,
                monster_spawn_block_light_limit,
                skybox,
                cardinal_light,
            },
            infiniburn_tag,
            monster_spawn_light_level,
            default_clock,
        },
        attributes,
        timelines,
        references,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DimensionSkybox {
    None,
    Overworld,
    End,
}

impl DimensionSkybox {
    fn decode(raw: &str) -> Option<Self> {
        match raw {
            "none" => Some(Self::None),
            "overworld" => Some(Self::Overworld),
            "end" => Some(Self::End),
            _ => None,
        }
    }

    fn serialized_name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Overworld => "overworld",
            Self::End => "end",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DimensionCardinalLight {
    Default,
    Nether,
}

impl DimensionCardinalLight {
    fn decode(raw: &str) -> Option<Self> {
        match raw {
            "default" => Some(Self::Default),
            "nether" => Some(Self::Nether),
            _ => None,
        }
    }

    fn serialized_name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Nether => "nether",
        }
    }
}

fn decode_tag_identifier(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<String, RegistryCodecError> {
    context.charge_node(field)?;
    let raw = input
        .java_string_ref()
        .and_then(|raw| raw.units().strip_prefix(&[b'#' as u16]))
        .and_then(|units| {
            bbb_protocol::JavaString::with_utf8_units(units, RegistryResourceIdentifier::parse)
        })
        .ok_or_else(|| context.codec_error(field, "expected a tag identifier"))?;
    let identifier = raw.ok_or_else(|| context.codec_error(field, "invalid tag identifier"))?;
    Ok(identifier.into_string())
}

fn required_bool_field(
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<bool, RegistryCodecError> {
    context.decode_bool(required_field(input, name, field, context)?, name)
}

fn optional_skybox_field(
    input: RegistryElementValueRef<'_>,
    name: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<Option<DimensionSkybox>, RegistryCodecError> {
    let Some(value) = input.field(name) else {
        return Ok(None);
    };
    context.charge_node(name)?;
    let raw = value
        .utf8_string()
        .ok_or_else(|| context.codec_error(name, "skybox is not a string"))?;
    DimensionSkybox::decode(&raw)
        .map(Some)
        .ok_or_else(|| context.codec_error(name, format!("unknown skybox `{raw}`")))
}

fn optional_cardinal_light_field(
    input: RegistryElementValueRef<'_>,
    name: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<Option<DimensionCardinalLight>, RegistryCodecError> {
    let Some(value) = input.field(name) else {
        return Ok(None);
    };
    context.charge_node(name)?;
    let raw = value
        .utf8_string()
        .ok_or_else(|| context.codec_error(name, "cardinal light is not a string"))?;
    DimensionCardinalLight::decode(&raw)
        .map(Some)
        .ok_or_else(|| context.codec_error(name, format!("unknown cardinal light `{raw}`")))
}

fn parse_identifier(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &RegistryCodecDecodeContext<'_>,
) -> Result<RegistryResourceIdentifier, RegistryCodecError> {
    let raw = input
        .java_string_ref()
        .ok_or_else(|| context.codec_error(field, "expected a named registry holder"))?;
    RegistryResourceIdentifier::parse_java_string(raw)
        .ok_or_else(|| context.codec_error(field, format!("invalid resource identifier `{raw}`")))
}

fn is_empty_compound(value: &NbtSummaryValue) -> bool {
    matches!(value, NbtSummaryValue::Compound(fields) if fields.is_empty())
}

fn java_double_compare(left: f64, right: f64) -> std::cmp::Ordering {
    if left < right {
        return std::cmp::Ordering::Less;
    }
    if left > right {
        return std::cmp::Ordering::Greater;
    }
    let bits = |value: f64| {
        if value.is_nan() {
            f64::NAN.to_bits() as i64
        } else {
            value.to_bits() as i64
        }
    };
    bits(left).cmp(&bits(right))
}

#[cfg(test)]
mod tests {
    use crate::registry_codec::recursive_values::CanonicalRegistryCodecValue;
    use crate::{
        decode_client_known_pack_registry_json, decode_direct_registry_json,
        decode_network_registry_element,
    };
    use bbb_protocol::{encode_nbt_summary, NbtSummaryEntry};

    use super::*;

    fn server(json: &[u8]) -> RegistryElementData {
        decode_direct_registry_json(json).unwrap()
    }

    fn known(json: &[u8]) -> RegistryElementData {
        decode_client_known_pack_registry_json(json).unwrap()
    }

    fn network(value: &NbtSummaryValue) -> RegistryElementData {
        let wire = encode_nbt_summary(value, 8 * 1024 * 1024).unwrap();
        decode_network_registry_element(&wire).unwrap()
    }

    fn decode_data(
        data: &RegistryElementData,
    ) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
        let decoded = decode_dimension_type_element(data)?;
        Ok(CanonicalRegistryCodecValue::from_parts(
            decoded.projected_network_nbt(),
            decoded.references,
        ))
    }

    fn compound(fields: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
        NbtSummaryValue::Compound(fields)
    }

    fn string(value: &str) -> NbtSummaryValue {
        NbtSummaryValue::String(value.to_string().into())
    }

    fn int(value: i32) -> NbtSummaryValue {
        NbtSummaryValue::Int(value)
    }

    fn float(value: f32) -> NbtSummaryValue {
        NbtSummaryValue::Float(canonical_float_tag_bits(value))
    }

    fn double(value: f64) -> NbtSummaryValue {
        NbtSummaryValue::Double(canonical_double_tag_bits(value))
    }

    fn bool_tag(value: bool) -> NbtSummaryValue {
        NbtSummaryValue::Byte(i8::from(value))
    }

    fn baseline_network_nbt() -> NbtSummaryValue {
        compound(vec![
            entry("has_skylight", bool_tag(true)),
            entry("has_ceiling", bool_tag(false)),
            entry("has_ender_dragon_fight", bool_tag(false)),
            entry("coordinate_scale", double(1.0)),
            entry("min_y", int(0)),
            entry("height", int(384)),
            entry("logical_height", int(256)),
            entry("infiniburn", string("#minecraft:infiniburn_overworld")),
            entry("ambient_light", float(0.0)),
            entry("monster_spawn_light_level", int(7)),
            entry("monster_spawn_block_light_limit", int(0)),
        ])
    }

    fn overworld_direct_json() -> &'static [u8] {
        br##"{
          "ambient_light":0.0,
          "attributes":{
            "minecraft:audio/ambient_sounds":{
              "mood":{
                "block_search_extent":8,
                "offset":2.0,
                "sound":"minecraft:ambient.cave",
                "tick_delay":6000
              }
            },
            "minecraft:audio/background_music":{
              "creative":{"max_delay":24000,"min_delay":12000,"sound":"minecraft:music.creative"},
              "default":{"max_delay":24000,"min_delay":12000,"sound":"minecraft:music.game"}
            },
            "minecraft:gameplay/bed_rule":{
              "can_set_spawn":"always",
              "can_sleep":"when_dark",
              "error_message":{"translate":"block.minecraft.bed.no_sleep"}
            },
            "minecraft:gameplay/nether_portal_spawns_piglin":true,
            "minecraft:gameplay/respawn_anchor_works":false,
            "minecraft:visual/ambient_light_color":"#0a0a0a",
            "minecraft:visual/cloud_color":"#ccffffff",
            "minecraft:visual/cloud_height":192.33,
            "minecraft:visual/fog_color":"#c0d8ff",
            "minecraft:visual/sky_color":"#78a7ff"
          },
          "coordinate_scale":1.0,
          "default_clock":"minecraft:overworld",
          "has_ceiling":false,
          "has_ender_dragon_fight":false,
          "has_skylight":true,
          "height":384,
          "infiniburn":"#minecraft:infiniburn_overworld",
          "logical_height":384,
          "min_y":-64,
          "monster_spawn_block_light_limit":0,
          "monster_spawn_light_level":{"type":"minecraft:uniform","max_inclusive":7,"min_inclusive":0},
          "timelines":"#minecraft:in_overworld"
        }"##
    }

    #[test]
    fn direct_facts_match_builtin_overworld_and_normalize_short_tag_ids() {
        let expected = DirectDimensionTypeFacts {
            scalars: DimensionTypeScalars {
                has_fixed_time: false,
                has_skylight: true,
                has_ceiling: false,
                has_ender_dragon_fight: false,
                coordinate_scale: 1.0,
                min_y: -64,
                height: 384,
                logical_height: 384,
                ambient_light: 0.0,
                monster_spawn_block_light_limit: 0,
                skybox: DimensionSkybox::Overworld,
                cardinal_light: DimensionCardinalLight::Default,
            },
            default_clock: Some("minecraft:overworld".to_owned()),
            infiniburn_tag: "minecraft:infiniburn_overworld".to_string(),
            monster_spawn_light_level: IntProvider::Uniform {
                min_inclusive: 0,
                max_inclusive: 7,
            },
        };
        assert_eq!(
            decode_direct_dimension_type_facts(&server(overworld_direct_json())).unwrap(),
            expected
        );

        let short_tag = std::str::from_utf8(overworld_direct_json())
            .unwrap()
            .replace("#minecraft:infiniburn_overworld", "#infiniburn_overworld");
        assert_eq!(
            decode_direct_dimension_type_facts(&server(short_tag.as_bytes())).unwrap(),
            expected
        );
        assert_eq!(
            canonical_direct_dimension_type(&server(overworld_direct_json())).unwrap(),
            canonical_direct_dimension_type(&server(short_tag.as_bytes())).unwrap()
        );
    }

    #[test]
    fn direct_known_pack_and_network_sources_share_one_canonical_shape() {
        let json = br##"{
          "has_skylight":true,
          "has_ceiling":false,
          "has_ender_dragon_fight":false,
          "coordinate_scale":1.0,
          "min_y":0,
          "height":384,
          "logical_height":256,
          "infiniburn":"#minecraft:infiniburn_overworld",
          "ambient_light":0.0,
          "monster_spawn_light_level":7,
          "monster_spawn_block_light_limit":0
        }"##;
        let expected = baseline_network_nbt();
        for data in [server(json), known(json), network(&expected)] {
            let decoded = decode_data(&data).unwrap();
            assert_eq!(decoded.network_nbt_unchecked(), &expected);
        }
    }

    #[test]
    fn optional_defaults_are_omitted_and_non_defaults_keep_record_order() {
        let decoded = decode_data(&server(
            br##"{
              "has_fixed_time":true,
              "has_skylight":true,
              "has_ceiling":false,
              "has_ender_dragon_fight":false,
              "coordinate_scale":8.0,
              "min_y":-64,
              "height":384,
              "logical_height":256,
              "infiniburn":"#infiniburn_nether",
              "ambient_light":0.1,
              "monster_spawn_light_level":{"type":"uniform","min_inclusive":0,"max_inclusive":15},
              "monster_spawn_block_light_limit":15,
              "skybox":"end",
              "cardinal_light":"nether",
              "attributes":{"minecraft:visual/fog_color":"#112233"},
              "timelines":"example:timeline",
              "default_clock":"example:clock"
            }"##,
        ))
        .unwrap();
        assert_eq!(
            decoded.network_nbt_unchecked(),
            &compound(vec![
                entry("has_fixed_time", bool_tag(true)),
                entry("has_skylight", bool_tag(true)),
                entry("has_ceiling", bool_tag(false)),
                entry("has_ender_dragon_fight", bool_tag(false)),
                entry("coordinate_scale", double(8.0)),
                entry("min_y", int(-64)),
                entry("height", int(384)),
                entry("logical_height", int(256)),
                entry("infiniburn", string("#minecraft:infiniburn_nether")),
                entry("ambient_light", float(0.1)),
                entry(
                    "monster_spawn_light_level",
                    compound(vec![
                        entry("min_inclusive", int(0)),
                        entry("max_inclusive", int(15)),
                        entry("type", string("minecraft:uniform")),
                    ]),
                ),
                entry("monster_spawn_block_light_limit", int(15)),
                entry("skybox", string("end")),
                entry("cardinal_light", string("nether")),
                entry(
                    "attributes",
                    compound(vec![
                        entry("minecraft:visual/fog_color", string("#112233"),)
                    ]),
                ),
                entry("timelines", string("example:timeline")),
                entry("default_clock", string("example:clock")),
            ])
        );
    }

    #[test]
    fn network_writer_filters_direct_and_network_inputs_but_keeps_filtered_references() {
        let json = br##"{
              "has_skylight":true,
              "has_ceiling":false,
              "has_ender_dragon_fight":false,
              "coordinate_scale":1.0,
              "min_y":0,
              "height":384,
              "logical_height":256,
              "infiniburn":"#minecraft:infiniburn_overworld",
              "ambient_light":0.0,
              "monster_spawn_light_level":7,
              "monster_spawn_block_light_limit":0,
              "attributes":{
                "minecraft:gameplay/bed_rule":{
                  "can_sleep":"always",
                  "can_set_spawn":"always",
                  "error_message":{
                    "text":"x",
                    "click_event":{"action":"show_dialog","dialog":"example:dialog"}
                  }
                }
              }
            }"##;
        for data in [server(json), known(json)] {
            let decoded = decode_data(&data).unwrap();
            assert_eq!(decoded.network_nbt_unchecked(), &baseline_network_nbt());
            let references = decoded.references().collect::<Vec<_>>();
            assert_eq!(references.len(), 1);
            assert_eq!(references[0].target_registry, "minecraft:dialog");
            assert_eq!(references[0].resource_id.as_str(), "example:dialog");
        }
    }
}
