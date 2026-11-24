use crate::persistent_component_nbt::source_codec::decode_data_component_patch_source;
use crate::registry_codec::{
    recursive_values::{required_field, CanonicalRegistryCodecValue, RegistryCodecDecodeContext},
    RegistryCodecError, RegistryElementValueRef,
};
use bbb_facts::item_types::vanilla_item_type_for_resource_id;
use bbb_protocol::{nbt_summary::entry, NbtSummaryValue};

use super::decode_identifier;

/// Decodes Java 26.1 `ItemStackTemplate.CODEC` and always emits the primary
/// `MAP_CODEC` writer form. This is raw template decoding: it intentionally
/// does not apply prototype sanitation, effective-stack validation, or the
/// concrete item's maximum stack size (`ItemStackTemplate`, with
/// construction and later strict validation separated at
/// `ItemStackTemplate` and `ItemStackTemplate`).
pub(crate) fn decode_item_stack_template_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    if input.utf8_string().is_some() {
        let item = decode_item_identifier(input, field, context)?;
        Ok(finish_template(item, 1, None))
    } else {
        decode_map(input, field, context)
    }
}

fn decode_map(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "item stack template")?;

    let id_field = format!("{field}.id");
    let id = required_field(input, "id", field, context)?;
    let item = decode_item_identifier(id, &id_field, context)?;
    let count = match input.field("count") {
        Some(value) => {
            let count_field = format!("{field}.count");
            context.charge_node(&count_field)?;
            let count = value
                .codec_i32()
                .ok_or_else(|| context.codec_error(&count_field, "expected Codec.INT input"))?;
            if !(1..=99).contains(&count) {
                return Err(context.codec_error(&count_field, "count is outside 1..=99"));
            }
            count
        }
        None => 1,
    };

    let components = match input.field("components") {
        Some(value) => {
            let components_field = format!("{field}.components");
            Some(context.with_nested(&components_field, |context| {
                decode_data_component_patch_source(value, &components_field, context)
            })?)
        }
        None => None,
    };

    Ok(finish_template(item, count, components))
}

fn finish_template(
    item: String,
    count: i32,
    components: Option<CanonicalRegistryCodecValue>,
) -> CanonicalRegistryCodecValue {
    let mut output = vec![entry("id", NbtSummaryValue::String(item.into()))];
    if count != 1 {
        output.push(entry("count", NbtSummaryValue::Int(count)));
    }
    let references = if let Some(components) = components {
        let (components, nested_references) = components.into_parts();
        if matches!(&components, NbtSummaryValue::Compound(entries) if entries.is_empty()) {
            Vec::new()
        } else {
            output.push(entry("components", components));
            nested_references
        }
    } else {
        Vec::new()
    };
    CanonicalRegistryCodecValue::from_parts(NbtSummaryValue::Compound(output), references)
}

fn decode_item_identifier(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<String, RegistryCodecError> {
    let identifier = decode_identifier(input, field, context)?.into_string();
    if identifier == "minecraft:air" {
        return Err(context.codec_error(field, "item must not be minecraft:air"));
    }
    if vanilla_item_type_for_resource_id(&identifier).is_none()
        && context
            .content_registry()
            .item_by_resource_id(&identifier)
            .is_none()
    {
        return Err(context.codec_error(field, format!("unknown built-in item {identifier}")));
    }
    Ok(identifier)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_parameter_reads_count_from_source_and_ignores_unconsumed_values() {
        let data = crate::decode_direct_registry_json(
            br#"{"id":"stone","count":2.75,"ignored":[null,{},false]}"#,
        )
        .unwrap();
        assert_eq!(
            crate::decode_item_stack_template_parameter_source(
                data.root(),
                data.source,
                Default::default(),
                Default::default(),
            )
            .unwrap(),
            NbtSummaryValue::Compound(vec![
                entry("id", NbtSummaryValue::String("minecraft:stone".into())),
                entry("count", NbtSummaryValue::Int(2)),
            ]),
        );
    }

    #[test]
    fn template_parameter_keeps_ignored_envelope_and_component_budgets_separate() {
        let envelope_limits = crate::PersistentNbtDecodeLimits {
            max_depth: 127,
            max_nodes: usize::MAX,
            max_collection: usize::MAX,
        };
        let component_limits = crate::PersistentNbtDecodeLimits {
            max_depth: 64,
            max_nodes: usize::MAX,
            max_collection: 4096,
        };
        let wide = vec!["0"; 4097].join(",");
        let ignored = format!("{}0{}", "[".repeat(80), "]".repeat(80));
        let raw = format!(r#"{{"id":"stone","ignored":{ignored},"wide":[{wide}]}}"#);
        let data = crate::decode_loaded_registry_json(raw.as_bytes(), raw.len()).unwrap();
        assert!(crate::decode_item_stack_template_parameter_source(
            data.root(),
            data.source,
            envelope_limits,
            component_limits,
        )
        .is_ok());
        let raw = format!(r#"{{"id":"stone","components":{{"custom_data":{{"wide":[{wide}]}}}}}}"#);
        let data = crate::decode_loaded_registry_json(raw.as_bytes(), raw.len()).unwrap();
        assert!(crate::decode_item_stack_template_parameter_source(
            data.root(),
            data.source,
            envelope_limits,
            component_limits,
        )
        .is_err());
    }
}
