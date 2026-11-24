//! Exact source-aware owner for Vanilla 26.1 `Timeline`.
//!
//! `Timeline.NETWORK_CODEC` decodes the full direct value and only then drops
//! non-syncable tracks. This owner keeps the filtered tracks' references so
//! the frozen candidate can still validate named holders before publication.

use std::collections::HashSet;

use crate::registry_codec::{
    holders::decode_fixed_holder,
    recursive_values::{
        required_field, CanonicalRegistryCodecValue, OwnedRegistryCodecReference,
        RegistryCodecDecodeContext,
    },
    synchronized_registry_element_codec, RegistryCodecError, RegistryElementData,
    RegistryElementValueRef, RegistryResourceIdentifier,
};
use bbb_facts::environment_attribute_types::{
    vanilla_environment_attribute, VanillaEnvironmentAttribute,
};
use bbb_protocol::{
    nbt_summary::{canonical_float_tag_bits, entry},
    NbtSummaryValue,
};

use super::environment_source::{decode_attribute_value_source, decode_modifier_argument_source};

const TIMELINE_REGISTRY: &str = "minecraft:timeline";
const WORLD_CLOCK_REGISTRY: &str = "minecraft:world_clock";
const SIMPLE_EASINGS: &[&str] = &[
    "constant",
    "linear",
    "in_back",
    "in_bounce",
    "in_circ",
    "in_cubic",
    "in_elastic",
    "in_expo",
    "in_quad",
    "in_quart",
    "in_quint",
    "in_sine",
    "in_out_back",
    "in_out_bounce",
    "in_out_circ",
    "in_out_cubic",
    "in_out_elastic",
    "in_out_expo",
    "in_out_quad",
    "in_out_quart",
    "in_out_quint",
    "in_out_sine",
    "out_back",
    "out_bounce",
    "out_circ",
    "out_cubic",
    "out_elastic",
    "out_expo",
    "out_quad",
    "out_quart",
    "out_quint",
    "out_sine",
];

pub(crate) fn decode_timeline_element_with_content_registry(
    data: &RegistryElementData,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let codec = synchronized_registry_element_codec(TIMELINE_REGISTRY, data.source)?;
    let mut context = RegistryCodecDecodeContext::new(TIMELINE_REGISTRY, codec, data.source)
        .with_content_registry(content_registry);
    decode_timeline(data.root(), "<root>", &mut context, true)
}

#[cfg(test)]
pub(crate) fn decode_timeline_element(
    data: &RegistryElementData,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    decode_timeline_element_with_content_registry(
        data,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

// Timeline: server samplers consume DIRECT tracks before network filtering.
pub fn canonical_direct_timeline(
    data: &RegistryElementData,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    canonical_direct_timeline_with_content_registry(
        data,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

pub fn canonical_direct_timeline_with_content_registry(
    data: &RegistryElementData,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    let codec = synchronized_registry_element_codec(TIMELINE_REGISTRY, data.source)?;
    let mut context = RegistryCodecDecodeContext::new(TIMELINE_REGISTRY, codec, data.source)
        .with_content_registry(content_registry);
    decode_timeline(data.root(), "<root>", &mut context, false)
        .map(|decoded| decoded.into_parts().0)
}

fn decode_timeline(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    filter_non_syncable: bool,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "timeline")?;
    context.charge_node("clock")?;
    let clock = decode_fixed_holder(
        TIMELINE_REGISTRY,
        context.owner_codec(),
        "clock",
        required_field(input, "clock", field, context)?,
    )?;
    let period_ticks = match input.field("period_ticks") {
        Some(value) => Some(context.decode_i32_range(value, "period_ticks", 1, i32::MAX)?),
        None => None,
    };
    let tracks = match input.field("tracks") {
        Some(value) => Some(context.with_nested("tracks", |context| {
            decode_tracks(value, "tracks", context, period_ticks, filter_non_syncable)
        })?),
        None => None,
    };
    let time_markers = match input.field("time_markers") {
        Some(value) => Some(context.with_nested("time_markers", |context| {
            decode_time_markers(value, "time_markers", context, period_ticks)
        })?),
        None => None,
    };

    let mut fields = vec![entry(
        "clock",
        NbtSummaryValue::String(clock.as_str().into()),
    )];
    if let Some(period_ticks) = period_ticks {
        fields.push(entry("period_ticks", NbtSummaryValue::Int(period_ticks)));
    }

    let mut references = vec![OwnedRegistryCodecReference::element(
        WORLD_CLOCK_REGISTRY,
        clock,
    )];
    if let Some(tracks) = tracks {
        let (tracks_value, track_references) = tracks.into_parts();
        if !matches!(&tracks_value, NbtSummaryValue::Compound(entries) if entries.is_empty()) {
            fields.push(entry("tracks", tracks_value));
        }
        references.extend(track_references);
    }
    if let Some(time_markers) = time_markers {
        let (time_markers_value, time_marker_references) = time_markers.into_parts();
        if !matches!(&time_markers_value, NbtSummaryValue::Compound(entries) if entries.is_empty())
        {
            fields.push(entry("time_markers", time_markers_value));
        }
        references.extend(time_marker_references);
    }
    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::Compound(fields),
        references,
    ))
}

fn decode_tracks(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    period_ticks: Option<i32>,
    filter_non_syncable: bool,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let len = context.charge_record(input, field, "tracks")?;

    let mut seen = HashSet::with_capacity(len);
    let mut entries = Vec::with_capacity(len);
    let mut references = Vec::new();

    for index in 0..len {
        let (raw_attribute, value) = input
            .map_entry_java_string_ref(index)
            .expect("timeline track map length and entries stay consistent");
        let attribute = RegistryResourceIdentifier::parse_java_string(raw_attribute)
            .ok_or_else(|| context.codec_error(field, "invalid environment attribute id"))?;
        if !seen.insert(attribute.as_str().to_string()) {
            return Err(context.codec_error(
                field,
                format!("duplicate environment attribute {}", attribute.as_str()),
            ));
        }
        let descriptor = vanilla_environment_attribute(attribute.as_str()).ok_or_else(|| {
            context.codec_error(
                field,
                format!("unknown environment attribute {}", attribute.as_str()),
            )
        })?;
        let decoded = context.with_nested(field, |context| {
            decode_track(value, field, context, descriptor, period_ticks)
        })?;
        let (track_nbt, track_refs) = decoded.into_parts();
        references.extend(track_refs);
        if !filter_non_syncable || descriptor.syncable {
            entries.push(entry(attribute.as_str(), track_nbt));
        }
    }

    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::Compound(entries),
        references,
    ))
}

fn decode_track(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    descriptor: &VanillaEnvironmentAttribute,
    period_ticks: Option<i32>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "attribute track")?;
    let modifier = match input.field("modifier") {
        Some(value) => decode_modifier_name(value, "modifier", context, descriptor)?,
        None => "override",
    };
    let keyframes = context.with_nested("keyframes", |context| {
        decode_keyframes(
            required_field(input, "keyframes", field, context)?,
            "keyframes",
            context,
            descriptor,
            modifier,
            period_ticks,
        )
    })?;
    let easing = match input.field("ease") {
        Some(value) => {
            context.with_nested("ease", |context| decode_easing(value, "ease", context))?
        }
        None => None,
    };

    let (keyframes, mut references) = keyframes.into_parts();
    let mut fields = vec![entry("keyframes", keyframes)];
    if let Some(easing) = easing {
        let (easing, easing_refs) = easing.into_parts();
        fields.push(entry("ease", easing));
        references.extend(easing_refs);
    }
    if modifier != "override" {
        fields.push(entry("modifier", NbtSummaryValue::String(modifier.into())));
    }
    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::Compound(fields),
        references,
    ))
}

fn decode_keyframes(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    descriptor: &VanillaEnvironmentAttribute,
    modifier: &str,
    period_ticks: Option<i32>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    let keyframes = input
        .list_elements()
        .ok_or_else(|| context.codec_error(field, "keyframes are not a list"))?;
    let len = keyframes.len();
    context.check_collection(field, len)?;
    if len == 0 {
        return Err(context.codec_error(field, "keyframes must not be empty"));
    }

    let mut values = Vec::with_capacity(len);
    let mut references = Vec::new();
    let mut previous_ticks = None;
    let mut repeated_previous_tick = input
        .list_element(len - 1)
        .and_then(|value| value.field("ticks"))
        .and_then(RegistryElementValueRef::codec_i32);
    let mut repeat_count = 0usize;

    for keyframe in keyframes {
        let decoded = context.with_nested(field, |context| {
            decode_keyframe(keyframe, field, context, descriptor, modifier, period_ticks)
        })?;
        let ticks = decoded.ticks;
        if previous_ticks.is_some_and(|previous| ticks < previous) {
            return Err(context.codec_error("ticks", "keyframes must be ordered by ticks"));
        }
        match repeated_previous_tick {
            Some(previous) if ticks == previous => {
                repeat_count += 1;
                if repeat_count > 2 {
                    return Err(context.codec_error(
                        "ticks",
                        format!("more than 2 keyframes share tick {ticks}"),
                    ));
                }
            }
            _ => repeat_count = 0,
        }
        previous_ticks = Some(ticks);
        repeated_previous_tick = Some(ticks);
        values.push(decoded.value);
        references.extend(decoded.references);
    }

    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::List(values),
        references,
    ))
}

struct DecodedKeyframe {
    ticks: i32,
    value: NbtSummaryValue,
    references: Vec<OwnedRegistryCodecReference>,
}

fn decode_keyframe(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    descriptor: &VanillaEnvironmentAttribute,
    modifier: &str,
    period_ticks: Option<i32>,
) -> Result<DecodedKeyframe, RegistryCodecError> {
    context.charge_record(input, field, "keyframe")?;
    let ticks = context.decode_i32_range(
        required_field(input, "ticks", field, context)?,
        "ticks",
        0,
        i32::MAX,
    )?;
    if let Some(period_ticks) = period_ticks {
        if ticks > period_ticks {
            return Err(
                context.codec_error("ticks", format!("must be in range [0; {period_ticks}]"))
            );
        }
    }
    let decoded = context.with_nested("value", |context| {
        let value = required_field(input, "value", field, context)?;
        if modifier == "override" {
            decode_attribute_value_source(descriptor, value, "value", context)
        } else {
            decode_modifier_argument_source(descriptor, modifier, value, "value", context)
        }
    })?;
    let (value, references) = decoded.into_parts();
    Ok(DecodedKeyframe {
        ticks,
        value: NbtSummaryValue::Compound(vec![
            entry("ticks", NbtSummaryValue::Int(ticks)),
            entry("value", value),
        ]),
        references,
    })
}

fn decode_easing(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<Option<CanonicalRegistryCodecValue>, RegistryCodecError> {
    if let Some(name) = input.java_string_ref() {
        context.charge_node(field)?;
        if !SIMPLE_EASINGS.iter().any(|allowed| name == *allowed) {
            return Err(context.codec_error(field, format!("unknown easing name `{name}`")));
        }
        if name == "linear" {
            return Ok(None);
        }
        return Ok(Some(CanonicalRegistryCodecValue::new(
            NbtSummaryValue::String(name.clone()),
        )));
    }

    context.charge_record(input, field, "easing")?;
    let values = required_field(input, "cubic_bezier", field, context)?;
    context.charge_node("cubic_bezier")?;
    let elements = values
        .list_elements()
        .ok_or_else(|| context.codec_error("cubic_bezier", "expected four-float list"))?;
    if elements.len() != 4 {
        return Err(context.codec_error("cubic_bezier", "expected four-float list"));
    }
    let mut controls = Vec::with_capacity(4);
    for (index, control) in elements.enumerate() {
        let value = context.decode_f32(control, "cubic_bezier")?;
        if matches!(index, 0 | 2) && (value < 0.0 || value > 1.0) {
            let axis = if index == 0 { "x1" } else { "x2" };
            return Err(
                context.codec_error("cubic_bezier", format!("{axis} must be in range [0; 1]"))
            );
        }
        controls.push(NbtSummaryValue::Float(canonical_float_tag_bits(value)));
    }
    Ok(Some(CanonicalRegistryCodecValue::new(
        NbtSummaryValue::Compound(vec![entry("cubic_bezier", NbtSummaryValue::List(controls))]),
    )))
}

fn decode_time_markers(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    period_ticks: Option<i32>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let len = context.charge_record(input, field, "time markers")?;
    let mut seen = HashSet::with_capacity(len);
    let mut entries = Vec::with_capacity(len);
    for index in 0..len {
        let (raw_marker, value) = input
            .map_entry_java_string_ref(index)
            .expect("time marker map length and entries stay consistent");
        let marker = RegistryResourceIdentifier::parse_java_string(raw_marker)
            .ok_or_else(|| context.codec_error(field, "invalid time marker id"))?;
        if !seen.insert(marker.as_str().to_string()) {
            return Err(
                context.codec_error(field, format!("duplicate time marker {}", marker.as_str()))
            );
        }
        let marker_value = context.with_nested(field, |context| {
            decode_time_marker(value, field, context, period_ticks)
        })?;
        entries.push(entry(marker.as_str(), marker_value));
    }
    Ok(CanonicalRegistryCodecValue::new(NbtSummaryValue::Compound(
        entries,
    )))
}

fn decode_time_marker(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    period_ticks: Option<i32>,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    if input.map_len().is_none() {
        let ticks = context.decode_i32_range(input, field, 0, i32::MAX)?;
        validate_time_marker_period(ticks, field, context, period_ticks)?;
        return Ok(NbtSummaryValue::Int(ticks));
    }

    context.charge_record(input, field, "time marker")?;
    let ticks = context.decode_i32_range(
        required_field(input, "ticks", field, context)?,
        "ticks",
        0,
        i32::MAX,
    )?;
    validate_time_marker_period(ticks, field, context, period_ticks)?;
    let show_in_commands = match input.field("show_in_commands") {
        Some(value) => context.decode_bool(value, "show_in_commands")?,
        None => false,
    };
    if !show_in_commands {
        return Ok(NbtSummaryValue::Int(ticks));
    }
    Ok(NbtSummaryValue::Compound(vec![
        entry("ticks", NbtSummaryValue::Int(ticks)),
        entry("show_in_commands", NbtSummaryValue::Byte(1)),
    ]))
}

fn validate_time_marker_period(
    ticks: i32,
    field: &str,
    context: &RegistryCodecDecodeContext<'_>,
    period_ticks: Option<i32>,
) -> Result<(), RegistryCodecError> {
    if let Some(period_ticks) = period_ticks {
        if ticks >= period_ticks {
            return Err(
                context.codec_error(field, format!("ticks must be in range [0; {period_ticks})"))
            );
        }
    }
    Ok(())
}

fn decode_modifier_name(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    descriptor: &VanillaEnvironmentAttribute,
) -> Result<&'static str, RegistryCodecError> {
    context.charge_node(field)?;
    let modifier = input
        .java_string_ref()
        .ok_or_else(|| context.codec_error(field, "modifier is not a string"))?;
    modifier
        .with_utf8(|modifier| modifier_allowed(descriptor, modifier))
        .flatten()
        .ok_or_else(|| context.codec_error(field, format!("modifier `{modifier}` is not allowed")))
}

fn modifier_allowed(
    descriptor: &VanillaEnvironmentAttribute,
    modifier: &str,
) -> Option<&'static str> {
    if modifier == "override" {
        return Some("override");
    }
    let allowed = match descriptor.value_type.modifier_family() {
        bbb_facts::environment_attribute_types::VanillaEnvironmentModifierFamily::Boolean => {
            &["and", "nand", "or", "nor", "xor", "xnor"][..]
        }
        bbb_facts::environment_attribute_types::VanillaEnvironmentModifierFamily::Float => &[
            "alpha_blend",
            "add",
            "subtract",
            "multiply",
            "minimum",
            "maximum",
        ][..],
        bbb_facts::environment_attribute_types::VanillaEnvironmentModifierFamily::Rgb
        | bbb_facts::environment_attribute_types::VanillaEnvironmentModifierFamily::Argb => &[
            "alpha_blend",
            "add",
            "subtract",
            "multiply",
            "blend_to_gray",
        ][..],
        bbb_facts::environment_attribute_types::VanillaEnvironmentModifierFamily::Integer => {
            &["add", "subtract", "multiply", "minimum", "maximum"][..]
        }
        bbb_facts::environment_attribute_types::VanillaEnvironmentModifierFamily::OverrideOnly => {
            &[][..]
        }
    };
    allowed.iter().copied().find(|allowed| *allowed == modifier)
}

#[cfg(test)]
mod tests;
