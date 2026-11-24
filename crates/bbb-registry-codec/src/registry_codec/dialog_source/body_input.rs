//! Source-aware `DialogBody` and `InputControl` codec owners.
//!
//! Vanilla 26.1 sources:
//! - `server/dialog/body/DialogBody`
//! - `server/dialog/body/ItemBody`
//! - `server/dialog/body/PlainMessage`
//! - `server/dialog/Input`
//! - `server/dialog/input/InputControl`
//! - `server/dialog/input/{BooleanInput,NumberRangeInput,SingleOptionInput,TextInput}.java`

use crate::component_source_codec::decode_component_source;
use crate::persistent_component_nbt::source_codec::decode_item_stack_template_source;
use std::cmp::Ordering;

use crate::registry_codec::recursive_values::{
    required_field, CanonicalRegistryCodecValue, RegistryCodecDecodeContext,
};
use bbb_protocol::{nbt_summary::canonical_float_tag_bits, NbtSummaryValue};

use super::{
    builtin_dispatch_id, decode_compact_list, decode_list, java_string_value, string_value,
    RegistryCodecError, RegistryCodecRecord, RegistryElementValueRef,
};

const BODY_TYPES: &[&str] = &["item", "plain_message"];
const INPUT_TYPES: &[&str] = &["boolean", "number_range", "single_option", "text"];

pub(super) fn decode_body_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    if !input.is_compound() {
        return Err(context.codec_error(field, "DialogBody dispatch expects a map"));
    }
    context.check_collection(field, input.map_len().unwrap())?;
    let raw_type = required_field(input, "type", field, context)?
        .utf8_string()
        .ok_or_else(|| context.codec_error("type", "expected a string dispatch type"))?;
    let (type_id, path) = builtin_dispatch_id(&raw_type, BODY_TYPES, "type", context)?;
    let object = match path.as_str() {
        "item" => decode_item_body(input, field, context)?,
        "plain_message" => decode_plain_message_map_payload(input, field, context)?,
        _ => unreachable!("builtin dispatch checked the body path"),
    };
    object.finish_dispatch(&type_id, field, context)
}

pub(super) fn decode_body_compact_list_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    decode_compact_list(input, field, context, |input, field, context| {
        decode_body_source(input, field, context)
    })
}

fn decode_item_body(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<RegistryCodecRecord, RegistryCodecError> {
    let mut object = RegistryCodecRecord::default();
    let item = required_field(input, "item", field, context)?;
    let item = context.with_nested("item", |context| {
        decode_item_stack_template_source(item, "item", context)
    })?;
    object.push("item", item);

    if let Some(description) = input.field("description") {
        let description = decode_plain_message(description, "description", context)?;
        object.push("description", description);
    }
    optional_bool_field(input, "show_decorations", true, context, &mut object)?;
    optional_bool_field(input, "show_tooltip", true, context, &mut object)?;
    optional_i32_range_field(input, "width", 16, 1, 256, context, &mut object)?;
    optional_i32_range_field(input, "height", 16, 1, 256, context, &mut object)?;
    Ok(object)
}

fn decode_plain_message(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.with_nested(field, |context| {
        if input.field("contents").is_some() {
            context.charge_record(input, field, "plain message")?;
            return Ok(decode_plain_message_map_payload(input, field, context)?.finish());
        }
        let contents = decode_component_source(input, field, context)?;
        let mut object = RegistryCodecRecord::default();
        object.push("contents", contents);
        Ok(object.finish())
    })
}

/// Decodes the flattened primary `PlainMessage.MAP_CODEC` payload.
///
/// The caller owns the surrounding map node: it is either the body dispatch
/// map or the explicitly charged primary branch of `PlainMessage.CODEC`.
fn decode_plain_message_map_payload(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<RegistryCodecRecord, RegistryCodecError> {
    let mut object = RegistryCodecRecord::default();
    let contents = required_field(input, "contents", field, context)?;
    let contents = context.with_nested("contents", |context| {
        decode_component_source(contents, "contents", context)
    })?;
    object.push("contents", contents);
    optional_i32_range_field(input, "width", 200, 1, 1024, context, &mut object)?;
    Ok(object)
}

pub(super) fn decode_input_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    if !input.is_compound() {
        return Err(context.codec_error(field, "Input codec expects a map"));
    }
    context.check_collection(field, input.map_len().unwrap())?;

    let key = required_field(input, "key", field, context)?
        .utf8_string()
        .ok_or_else(|| context.codec_error("key", "expected an input-name string"))?;
    if !super::action::is_valid_variable_name(&key) {
        return Err(context.codec_error("key", format!("`{key}` is not a valid input name")));
    }
    let mut object = RegistryCodecRecord::default();
    object.push("key", string_value(&key, "key", context)?);

    let raw_type = required_field(input, "type", field, context)?
        .utf8_string()
        .ok_or_else(|| context.codec_error("type", "expected a string dispatch type"))?;
    let (type_id, path) = builtin_dispatch_id(&raw_type, INPUT_TYPES, "type", context)?;
    match path.as_str() {
        "boolean" => decode_boolean_input(input, field, context, &mut object)?,
        "number_range" => decode_number_range_input(input, field, context, &mut object)?,
        "single_option" => decode_single_option_input(input, field, context, &mut object)?,
        "text" => decode_text_input(input, field, context, &mut object)?,
        _ => unreachable!("builtin dispatch checked the input-control path"),
    }
    object.finish_dispatch(&type_id, field, context)
}

pub(super) fn decode_input_list_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    decode_list(input, field, context, |input, field, context| {
        decode_input_source(input, field, context)
    })
}

fn decode_boolean_input(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    object: &mut RegistryCodecRecord,
) -> Result<(), RegistryCodecError> {
    required_component_field(input, "label", field, context, object)?;
    optional_bool_field(input, "initial", false, context, object)?;
    optional_string_field(input, "on_true", "true", context, object)?;
    optional_string_field(input, "on_false", "false", context, object)?;
    Ok(())
}

fn decode_number_range_input(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    object: &mut RegistryCodecRecord,
) -> Result<(), RegistryCodecError> {
    optional_i32_range_field(input, "width", 200, 1, 1024, context, object)?;
    required_component_field(input, "label", field, context, object)?;
    optional_string_field(
        input,
        "label_format",
        "options.generic_value",
        context,
        object,
    )?;

    let start = required_f32_field(input, "start", field, context, object)?;
    let end = required_f32_field(input, "end", field, context, object)?;
    if let Some(initial_input) = input.field("initial") {
        let initial = context.decode_f32(initial_input, "initial")?;
        validate_initial_range(start, end, initial, "initial", context)?;
        object.push("initial", canonical_f32(initial));
    }
    if let Some(step_input) = input.field("step") {
        let step = context.decode_f32(step_input, "step")?;
        validate_positive_step(step, "step", context)?;
        object.push("step", canonical_f32(step));
    }
    Ok(())
}

fn decode_single_option_input(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    object: &mut RegistryCodecRecord,
) -> Result<(), RegistryCodecError> {
    optional_i32_range_field(input, "width", 200, 1, 1024, context, object)?;
    let options = required_field(input, "options", field, context)?;
    let mut initial_count = 0usize;
    let options = context.with_nested("options", |context| {
        decode_list(
            options,
            "options",
            context,
            |entry, entry_field, context| {
                let value =
                    decode_single_option_entry(entry, entry_field, context, &mut initial_count)?;
                Ok(value)
            },
        )
    })?;
    if matches!(options.network_nbt_unchecked(), bbb_protocol::NbtSummaryValue::List(values) if values.is_empty())
    {
        return Err(context.codec_error("options", "options list must not be empty"));
    }
    if initial_count > 1 {
        return Err(context.codec_error("options", "multiple initial option values"));
    }
    object.push("options", options);
    required_component_field(input, "label", field, context, object)?;
    optional_bool_field(input, "label_visible", true, context, object)?;
    Ok(())
}

fn decode_single_option_entry(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    initial_count: &mut usize,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    if input.is_string() {
        let id = decode_java_string(input, field, context)?;
        let mut object = RegistryCodecRecord::default();
        object.push("id", java_string_value(&id));
        return Ok(object.finish());
    }

    context.charge_node(field)?;
    if !input.is_compound() {
        return Err(context.codec_error(field, "single-option entry expects a string or map"));
    }
    context.check_collection(field, input.map_len().unwrap())?;
    let mut object = RegistryCodecRecord::default();
    let id = decode_java_string(required_field(input, "id", field, context)?, "id", context)?;
    object.push("id", java_string_value(&id));
    if let Some(display) = input.field("display") {
        let display = context.with_nested("display", |context| {
            decode_component_source(display, "display", context)
        })?;
        object.push("display", display);
    }
    if let Some(initial) = input.field("initial") {
        let initial = context.decode_bool(initial, "initial")?;
        if initial {
            *initial_count += 1;
            object.push("initial", canonical_bool(true));
        }
    }
    Ok(object.finish())
}

fn decode_text_input(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    object: &mut RegistryCodecRecord,
) -> Result<(), RegistryCodecError> {
    optional_i32_range_field(input, "width", 200, 1, 1024, context, object)?;
    required_component_field(input, "label", field, context, object)?;
    optional_bool_field(input, "label_visible", true, context, object)?;

    let initial = match input.field("initial") {
        Some(initial) => {
            let initial = decode_java_string(initial, "initial", context)?;
            let decoded = java_string_value(&initial);
            if !initial.is_empty() {
                object.push("initial", decoded);
            }
            initial
        }
        None => bbb_protocol::JavaString::from(""),
    };
    let max_length = match input.field("max_length") {
        Some(max_length) => {
            let value = context.decode_i32(max_length, "max_length")?;
            if value <= 0 {
                return Err(context.codec_error("max_length", "max_length must be positive"));
            }
            if value != 32 {
                object.push("max_length", canonical_i32(value));
            }
            value
        }
        None => 32,
    };
    if initial.units().len() > max_length as usize {
        return Err(context.codec_error(field, "default text UTF-16 length exceeds max_length"));
    }

    if let Some(multiline) = input.field("multiline") {
        let multiline = context.with_nested("multiline", |context| {
            decode_multiline(multiline, "multiline", context)
        })?;
        object.push("multiline", multiline);
    }
    Ok(())
}

fn decode_multiline(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    if !input.is_compound() {
        return Err(context.codec_error(field, "multiline options expect a map"));
    }
    context.check_collection(field, input.map_len().unwrap())?;
    let mut object = RegistryCodecRecord::default();
    optional_positive_i32_field(input, "max_lines", context, &mut object)?;
    optional_i32_range_no_default_field(input, "height", 1, 512, context, &mut object)?;
    Ok(object.finish())
}

fn required_component_field(
    input: RegistryElementValueRef<'_>,
    name: &'static str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    object: &mut RegistryCodecRecord,
) -> Result<(), RegistryCodecError> {
    let child = required_field(input, name, field, context)?;
    let child = context.with_nested(name, |context| {
        decode_component_source(child, name, context)
    })?;
    object.push(name, child);
    Ok(())
}

fn optional_bool_field(
    input: RegistryElementValueRef<'_>,
    name: &'static str,
    default: bool,
    context: &mut RegistryCodecDecodeContext<'_>,
    object: &mut RegistryCodecRecord,
) -> Result<bool, RegistryCodecError> {
    let Some(input) = input.field(name) else {
        return Ok(default);
    };
    let value = context.decode_bool(input, name)?;
    if value != default {
        object.push(name, canonical_bool(value));
    }
    Ok(value)
}

fn optional_string_field(
    input: RegistryElementValueRef<'_>,
    name: &'static str,
    default: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    object: &mut RegistryCodecRecord,
) -> Result<bbb_protocol::JavaString, RegistryCodecError> {
    let Some(input) = input.field(name) else {
        return Ok(default.into());
    };
    let value = decode_java_string(input, name, context)?;
    let decoded = java_string_value(&value);
    if value != default {
        object.push(name, decoded);
    }
    Ok(value)
}

fn optional_i32_range_field(
    input: RegistryElementValueRef<'_>,
    name: &'static str,
    default: i32,
    minimum: i32,
    maximum: i32,
    context: &mut RegistryCodecDecodeContext<'_>,
    object: &mut RegistryCodecRecord,
) -> Result<i32, RegistryCodecError> {
    let Some(input) = input.field(name) else {
        return Ok(default);
    };
    let value = context.decode_i32_range(input, name, minimum, maximum)?;
    if value != default {
        object.push(name, canonical_i32(value));
    }
    Ok(value)
}

fn optional_positive_i32_field(
    input: RegistryElementValueRef<'_>,
    name: &'static str,
    context: &mut RegistryCodecDecodeContext<'_>,
    object: &mut RegistryCodecRecord,
) -> Result<Option<i32>, RegistryCodecError> {
    optional_i32_range_no_default_field(input, name, 1, i32::MAX, context, object)
}

fn optional_i32_range_no_default_field(
    input: RegistryElementValueRef<'_>,
    name: &'static str,
    minimum: i32,
    maximum: i32,
    context: &mut RegistryCodecDecodeContext<'_>,
    object: &mut RegistryCodecRecord,
) -> Result<Option<i32>, RegistryCodecError> {
    let Some(input) = input.field(name) else {
        return Ok(None);
    };
    let value = context.decode_i32_range(input, name, minimum, maximum)?;
    object.push(name, canonical_i32(value));
    Ok(Some(value))
}

fn required_f32_field(
    input: RegistryElementValueRef<'_>,
    name: &'static str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    object: &mut RegistryCodecRecord,
) -> Result<f32, RegistryCodecError> {
    let input = required_field(input, name, field, context)?;
    let value = context.decode_f32(input, name)?;
    object.push(name, canonical_f32(value));
    Ok(value)
}

fn decode_java_string(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<bbb_protocol::JavaString, RegistryCodecError> {
    context.charge_node(field)?;
    input
        .java_string()
        .ok_or_else(|| context.codec_error(field, "expected Codec.STRING input"))
}

fn validate_initial_range(
    start: f32,
    end: f32,
    initial: f32,
    field: &str,
    context: &RegistryCodecDecodeContext<'_>,
) -> Result<(), RegistryCodecError> {
    let minimum = java_math_min(start, end) as f64;
    let maximum = java_math_max(start, end) as f64;
    let initial_double = initial as f64;
    if initial_double < minimum || initial_double > maximum {
        return Err(context.codec_error(
            field,
            format!("initial value {initial_double} is outside [{minimum}, {maximum}]"),
        ));
    }
    Ok(())
}

fn validate_positive_step(
    step: f32,
    field: &str,
    context: &RegistryCodecDecodeContext<'_>,
) -> Result<(), RegistryCodecError> {
    if java_float_compare(step, 0.0) != Ordering::Greater
        || java_float_compare(step, f32::MAX) == Ordering::Greater
    {
        return Err(context.codec_error(field, "step must be a positive finite float"));
    }
    Ok(())
}

fn canonical_bool(value: bool) -> CanonicalRegistryCodecValue {
    CanonicalRegistryCodecValue::new(NbtSummaryValue::Byte(i8::from(value)))
}

fn canonical_i32(value: i32) -> CanonicalRegistryCodecValue {
    CanonicalRegistryCodecValue::new(NbtSummaryValue::Int(value))
}

fn canonical_f32(value: f32) -> CanonicalRegistryCodecValue {
    CanonicalRegistryCodecValue::new(NbtSummaryValue::Float(canonical_float_tag_bits(value)))
}

fn java_float_compare(left: f32, right: f32) -> Ordering {
    if left < right {
        return Ordering::Less;
    }
    if left > right {
        return Ordering::Greater;
    }
    let comparable_bits = |value: f32| {
        if value.is_nan() {
            0x7fc0_0000_u32 as i32
        } else {
            value.to_bits() as i32
        }
    };
    comparable_bits(left).cmp(&comparable_bits(right))
}

fn java_math_min(left: f32, right: f32) -> f32 {
    if left.is_nan() || right.is_nan() {
        return f32::NAN;
    }
    if left == 0.0 && right == 0.0 {
        return if left.is_sign_negative() || right.is_sign_negative() {
            -0.0
        } else {
            0.0
        };
    }
    if left < right {
        left
    } else {
        right
    }
}

fn java_math_max(left: f32, right: f32) -> f32 {
    if left.is_nan() || right.is_nan() {
        return f32::NAN;
    }
    if left == 0.0 && right == 0.0 {
        return if left.is_sign_positive() || right.is_sign_positive() {
            0.0
        } else {
            -0.0
        };
    }
    if left > right {
        left
    } else {
        right
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry_codec::{
        decode_client_known_pack_registry_json, decode_direct_registry_json,
        decode_network_registry_element, RegistryElementData, RegistryElementSource,
    };
    use bbb_protocol::{encode_nbt_summary, NbtSummaryEntry};

    fn context(source: RegistryElementSource) -> RegistryCodecDecodeContext<'static> {
        RegistryCodecDecodeContext::new(
            super::super::DIALOG_REGISTRY,
            super::super::DIALOG_CODEC,
            source,
        )
    }

    fn direct(json: &str) -> RegistryElementData {
        decode_direct_registry_json(json.as_bytes()).unwrap()
    }

    fn known(json: &str) -> RegistryElementData {
        decode_client_known_pack_registry_json(json.as_bytes()).unwrap()
    }

    fn decode_body(data: &RegistryElementData) -> CanonicalRegistryCodecValue {
        decode_body_source(data.root(), "$", &mut context(data.source)).unwrap()
    }

    fn decode_input(data: &RegistryElementData) -> CanonicalRegistryCodecValue {
        decode_input_source(data.root(), "$", &mut context(data.source)).unwrap()
    }

    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.into(),
            value,
        }
    }

    fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
        NbtSummaryValue::Compound(entries)
    }

    fn names(value: &CanonicalRegistryCodecValue) -> Vec<&bbb_protocol::JavaString> {
        let NbtSummaryValue::Compound(entries) = value.network_nbt_unchecked() else {
            panic!("expected compound")
        };
        entries.iter().map(|entry| &entry.name).collect()
    }

    fn field<'a>(value: &'a NbtSummaryValue, name: &str) -> Option<&'a NbtSummaryValue> {
        let NbtSummaryValue::Compound(entries) = value else {
            return None;
        };
        entries
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| &entry.value)
    }

    #[test]
    fn body_variants_preserve_record_order_defaults_and_plain_message_alternatives() {
        let item = decode_body(&direct(
            r#"{
                "type":"item",
                "item":"minecraft:stick",
                "description":"tip",
                "show_decorations":true,
                "show_tooltip":false,
                "width":16,
                "height":17
            }"#,
        ));
        assert_eq!(
            names(&item),
            ["item", "description", "show_tooltip", "height", "type"]
        );
        assert_eq!(
            field(item.network_nbt_unchecked(), "description"),
            Some(&compound(vec![entry(
                "contents",
                NbtSummaryValue::String("tip".to_string().into())
            )]))
        );

        let primary = decode_body(&direct(
            r#"{"type":"item","item":"stick","description":{"contents":"full","width":201}}"#,
        ));
        assert_eq!(
            field(primary.network_nbt_unchecked(), "description"),
            Some(&compound(vec![
                entry(
                    "contents",
                    NbtSummaryValue::String("full".to_string().into())
                ),
                entry("width", NbtSummaryValue::Int(201)),
            ]))
        );

        let plain = decode_body(&known(
            r#"{"type":"plain_message","contents":"hello","width":200}"#,
        ));
        assert_eq!(names(&plain), ["contents", "type"]);
    }

    #[test]
    fn body_compact_list_encodes_zero_one_many_and_scalar_per_vanilla_semantics() {
        let zero = direct("[]");
        let zero =
            decode_body_compact_list_source(zero.root(), "$", &mut context(zero.source)).unwrap();
        assert_eq!(
            zero.network_nbt_unchecked(),
            &NbtSummaryValue::List(Vec::new())
        );

        let scalar_data = direct(r#"{"type":"plain_message","contents":"one"}"#);
        let scalar = decode_body_compact_list_source(
            scalar_data.root(),
            "$",
            &mut context(scalar_data.source),
        )
        .unwrap();
        assert!(matches!(
            scalar.network_nbt_unchecked(),
            NbtSummaryValue::Compound(_)
        ));

        let one_data = direct(r#"[{"type":"plain_message","contents":"one"}]"#);
        let one =
            decode_body_compact_list_source(one_data.root(), "$", &mut context(one_data.source))
                .unwrap();
        assert_eq!(one.network_nbt_unchecked(), scalar.network_nbt_unchecked());

        let many_data = direct(
            r#"[
                {"type":"plain_message","contents":"one"},
                {"type":"plain_message","contents":"two"}
            ]"#,
        );
        let many =
            decode_body_compact_list_source(many_data.root(), "$", &mut context(many_data.source))
                .unwrap();
        assert!(
            matches!(many.network_nbt_unchecked(), NbtSummaryValue::List(values) if values.len() == 2)
        );
    }

    #[test]
    fn all_input_controls_apply_defaults_alternatives_and_dispatch_order() {
        let boolean = decode_input(&direct(
            r#"{"key":"enabled_1","type":"boolean","label":"Enabled","initial":false,"on_true":"true","on_false":"false"}"#,
        ));
        assert_eq!(names(&boolean), ["key", "label", "type"]);

        let number = decode_input(&direct(
            r#"{
                "key":"amount","type":"number_range","width":201,"label":"Amount",
                "label_format":"value.format","start":10,"end":-10,"initial":2,"step":0.5
            }"#,
        ));
        assert_eq!(
            names(&number),
            [
                "key",
                "width",
                "label",
                "label_format",
                "start",
                "end",
                "initial",
                "step",
                "type"
            ]
        );

        let options = decode_input(&direct(
            r#"{
                "key":"choice","type":"single_option","options":[
                    "first", {"id":"second","display":"Second","initial":true}
                ],"label":"Choice","label_visible":false
            }"#,
        ));
        assert_eq!(
            names(&options),
            ["key", "options", "label", "label_visible", "type"]
        );
        let NbtSummaryValue::List(entries) =
            field(options.network_nbt_unchecked(), "options").unwrap()
        else {
            panic!("options must be a list")
        };
        assert_eq!(
            entries[0],
            compound(vec![entry(
                "id",
                NbtSummaryValue::String("first".to_string().into())
            )])
        );
        assert_eq!(
            entries[1],
            compound(vec![
                entry("id", NbtSummaryValue::String("second".to_string().into())),
                entry(
                    "display",
                    NbtSummaryValue::String("Second".to_string().into())
                ),
                entry("initial", NbtSummaryValue::Byte(1)),
            ])
        );

        let text = decode_input(&direct(
            r#"{
                "key":"message","type":"text","width":300,"label":"Message",
                "label_visible":false,"initial":"hello","max_length":40,
                "multiline":{"max_lines":3,"height":120}
            }"#,
        ));
        assert_eq!(
            names(&text),
            [
                "key",
                "width",
                "label",
                "label_visible",
                "initial",
                "max_length",
                "multiline",
                "type"
            ]
        );
    }

    #[test]
    fn input_list_and_all_three_sources_produce_canonical_network_nbt() {
        let list_data = known(
            r#"[
                {"key":"a","type":"boolean","label":"A"},
                {"key":"b","type":"text","label":"B"}
            ]"#,
        );
        let list = decode_input_list_source(list_data.root(), "$", &mut context(list_data.source))
            .unwrap();
        assert!(
            matches!(list.network_nbt_unchecked(), NbtSummaryValue::List(values) if values.len() == 2)
        );

        let network_value = compound(vec![
            entry("key", NbtSummaryValue::String("flag".to_string().into())),
            entry(
                "type",
                NbtSummaryValue::String("boolean".to_string().into()),
            ),
            entry("label", NbtSummaryValue::String("Flag".to_string().into())),
            // `Codec.BOOL` uses Number.byteValue() under NbtOps; 256 narrows to false.
            entry("initial", NbtSummaryValue::Int(256)),
        ]);
        let wire = encode_nbt_summary(&network_value, 8192).unwrap();
        let network = decode_network_registry_element(&wire).unwrap();
        let decoded = decode_input(&network);
        assert_eq!(names(&decoded), ["key", "label", "type"]);
    }
}
