//! Source-aware owner for `DialogAction.CODEC`.

use crate::{
    component_source_codec::decode_dialog_static_action_source,
    registry_codec::{
        recursive_values::{
            required_field, CanonicalRegistryCodecValue, RegistryCodecDecodeContext,
        },
        source_nbt::decode_compound_tag_dynamic_source,
        RegistryCodecError, RegistryElementValueRef,
    },
};
use bbb_protocol::{java_char_is_letter_or_digit, normalize_resource_location};

use super::{builtin_dispatch_id, string_value, RegistryCodecRecord};

const ACTION_TYPES: [&str; 9] = [
    "open_url",
    "run_command",
    "suggest_command",
    "show_dialog",
    "change_page",
    "copy_to_clipboard",
    "custom",
    "dynamic/run_command",
    "dynamic/custom",
];

/// Decodes `Action.CODEC`, retaining both its NbtOps projection and exact
/// JsonOps writer length.
///
/// Vanilla dispatches through the built-in dialog-action-type registry and
/// writes the selected map codec before its normalized discriminator.
/// Static actions stay with the
/// Component click-event owner so validation, dialog-holder recursion, and
/// ordered references have one implementation.
pub(super) fn decode_action_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let type_field = format!("{field}.type");
    let raw_type = required_field(input, "type", field, context)?
        .utf8_string()
        .ok_or_else(|| context.codec_error(&type_field, "expected a string"))?;
    let (type_id, action_type) =
        builtin_dispatch_id(&raw_type, &ACTION_TYPES, &type_field, context)?;

    if !action_type.starts_with("dynamic/") {
        return decode_dialog_static_action_source(input, field, context);
    }

    context.charge_node(field)?;
    if !input.is_compound() {
        return Err(context.codec_error(field, "dialog action is not a map"));
    }
    context.check_collection(field, input.map_len().unwrap())?;

    let mut output = RegistryCodecRecord::default();
    match action_type.as_str() {
        "dynamic/run_command" => {
            let template_field = format!("{field}.template");
            let template = required_field(input, "template", field, context)?
                .java_string()
                .ok_or_else(|| context.codec_error(&template_field, "expected a string"))?;
            validate_parsed_template_units(template.units())
                .map_err(|reason| context.codec_error(&template_field, reason))?;
            context.charge_node(&template_field)?;
            output.push("template", super::java_string_value(&template));
        }
        "dynamic/custom" => {
            let id_field = format!("{field}.id");
            let raw_id = required_field(input, "id", field, context)?
                .utf8_string()
                .ok_or_else(|| context.codec_error(&id_field, "expected a string"))?;
            let id = normalize_resource_location(raw_id.as_str()).map_err(|_| {
                context.codec_error(&id_field, format!("invalid resource identifier `{raw_id}`"))
            })?;
            output.push("id", string_value(&id, &id_field, context)?);
            if let Some(additions) = input.field("additions") {
                let additions_field = format!("{field}.additions");
                // `CustomAll` uses bare `CompoundTag.CODEC`, whose passthrough
                // conversion accepts only a map here; unlike
                // `CustomData.COMPOUND_TAG_CODEC`, it has no flattened-SNBT
                // alternative.
                if additions.java_string().is_some() {
                    return Err(context.codec_error(
                        &additions_field,
                        "dynamic custom additions is not a compound",
                    ));
                }
                let additions = context.with_nested(&additions_field, |context| {
                    decode_compound_tag_dynamic_source(additions, &additions_field, context)
                })?;
                output.push("additions", additions);
            }
        }
        _ => unreachable!("the allow-list admitted only two dynamic actions"),
    }
    output.finish_dispatch(&type_id, field, context)
}

fn validate_parsed_template_units(units: &[u16]) -> Result<(), &'static str> {
    let mut index = 0;
    let mut saw_variable = false;
    while index < units.len() {
        if units[index] != u16::from(b'$') {
            index += 1;
            continue;
        }
        if units.get(index + 1) != Some(&u16::from(b'(')) {
            index += 1;
            continue;
        }
        let variable_start = index + 2;
        let Some(relative_end) = units[variable_start..]
            .iter()
            .position(|unit| *unit == u16::from(b')'))
        else {
            return Err("unterminated template variable");
        };
        let variable_end = variable_start + relative_end;
        if !units[variable_start..variable_end]
            .iter()
            .all(|&unit| unit == u16::from(b'_') || java_char_is_letter_or_digit(unit))
        {
            return Err("invalid template variable name");
        }
        saw_variable = true;
        index = variable_end + 1;
    }
    if !saw_variable {
        return Err("template has no variables");
    }
    Ok(())
}

/// Exact Java `StringTemplate.isValidVariableName` predicate shared with
/// dialog input keys. Vanilla accepts the empty string here; callers whose
/// surrounding grammar requires a non-empty name enforce that separately.
pub(super) fn is_valid_variable_name(value: &str) -> bool {
    value
        .encode_utf16()
        .all(|unit| unit == u16::from(b'_') || java_char_is_letter_or_digit(unit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry_codec::{
        decode_client_known_pack_registry_json, decode_direct_registry_json,
        recursive_values::CanonicalRegistryCodecValue, RegistryElementSource,
    };
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

    const OWNER: &str = "minecraft:dialog";
    const CODEC: &str = "Dialog.DIRECT_CODEC";

    fn context(source: RegistryElementSource) -> RegistryCodecDecodeContext<'static> {
        RegistryCodecDecodeContext::new(OWNER, CODEC, source)
    }

    fn decode_json(source: &[u8]) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
        let data = decode_direct_registry_json(source).unwrap();
        decode_action_source(data.root(), "$", &mut context(data.source))
    }

    fn compound(value: &NbtSummaryValue) -> &[NbtSummaryEntry] {
        let NbtSummaryValue::Compound(entries) = value else {
            panic!("expected compound, got {value:?}");
        };
        entries
    }

    fn field<'a>(value: &'a NbtSummaryValue, name: &str) -> &'a NbtSummaryValue {
        &compound(value)
            .iter()
            .find(|entry| entry.name == name)
            .unwrap()
            .value
    }

    #[test]
    fn decodes_all_nine_registered_action_types_and_normalizes_dispatch_ids() {
        let sources: [(&str, &str); 9] = [
            (
                r#"{"type":"open_url","url":"https://example.com"}"#,
                "open_url",
            ),
            (
                r#"{"type":"minecraft:run_command","command":"say hi"}"#,
                "run_command",
            ),
            (
                r#"{"type":"suggest_command","command":"say hi"}"#,
                "suggest_command",
            ),
            (
                r#"{"type":"show_dialog","dialog":"example:next"}"#,
                "show_dialog",
            ),
            (r#"{"type":"change_page","page":2}"#, "change_page"),
            (
                r#"{"type":"copy_to_clipboard","value":"copy"}"#,
                "copy_to_clipboard",
            ),
            (r#"{"type":"custom","id":"example:static"}"#, "custom"),
            (
                r#"{"type":"dynamic/run_command","template":"say $(name)"}"#,
                "dynamic/run_command",
            ),
            (
                r#"{"type":"minecraft:dynamic/custom","id":"example:dynamic"}"#,
                "dynamic/custom",
            ),
        ];
        for (source, expected_path) in sources {
            let decoded = decode_json(source.as_bytes()).unwrap();
            let entries = compound(decoded.network_nbt_unchecked());
            assert_eq!(
                entries.last(),
                Some(&NbtSummaryEntry {
                    name: "type".into(),
                    value: NbtSummaryValue::String(format!("minecraft:{expected_path}").into()),
                }),
                "{source}"
            );
        }
    }

    #[test]
    fn dynamic_custom_additions_pair_both_json_sources_and_network_nbt() {
        let json_map = decode_direct_registry_json(
            br#"{"type":"dynamic/custom","id":"event","additions":{"flag":true,"number":1e1}}"#,
        )
        .unwrap();
        let known_pack = decode_client_known_pack_registry_json(
            br#"{"type":"dynamic/custom","id":"event","additions":{"flag":true,"number":1e1}}"#,
        )
        .unwrap();
        let network = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "type".into(),
                value: NbtSummaryValue::String("minecraft:dynamic/custom".to_string().into()),
            },
            NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::String("event".to_string().into()),
            },
            NbtSummaryEntry {
                name: "additions".into(),
                value: NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "flag".into(),
                        value: NbtSummaryValue::Byte(1),
                    },
                    NbtSummaryEntry {
                        name: "number".into(),
                        value: NbtSummaryValue::Byte(10),
                    },
                ]),
            },
        ]);

        let mut values = Vec::new();
        for data in [&json_map, &known_pack] {
            values.push(decode_action_source(data.root(), "$", &mut context(data.source)).unwrap());
        }
        values.push(
            decode_action_source(
                RegistryElementValueRef::Nbt(&network),
                "$",
                &mut context(RegistryElementSource::NetworkNbt),
            )
            .unwrap(),
        );

        for value in &values {
            assert_eq!(
                field(value.network_nbt_unchecked(), "additions"),
                &NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "flag".into(),
                        value: NbtSummaryValue::Byte(1),
                    },
                    NbtSummaryEntry {
                        name: "number".into(),
                        value: NbtSummaryValue::Byte(10),
                    },
                ])
            );
        }
    }
}
