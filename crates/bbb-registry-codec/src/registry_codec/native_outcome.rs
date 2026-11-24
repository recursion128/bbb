//! Raw input budget checks shared by registry source codecs.

use super::{
    recursive_values::RegistryCodecDecodeContext, RegistryCodecError, RegistryElementValueRef,
};

pub fn validate_registry_source_input(
    input: RegistryElementValueRef<'_>,
    limits: crate::PersistentNbtDecodeLimits,
) -> Result<(), RegistryCodecError> {
    let source = match input {
        RegistryElementValueRef::Json(_) => crate::RegistryElementSource::ServerDatapackJson,
        _ => crate::RegistryElementSource::NetworkNbt,
    };
    let mut context =
        RegistryCodecDecodeContext::with_limits("<source>", "JsonOps", source, limits)?;
    let max_depth = inspect_raw_input(input, "<root>", &mut context)?;
    if max_depth > limits.max_depth {
        return Err(context.budget_error(
            "<root>",
            format!("container nesting exceeds depth {}", limits.max_depth),
        ));
    }
    Ok(())
}

pub(crate) fn validate_raw_input(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<(), RegistryCodecError> {
    inspect_raw_input(input, field, context).map(|_| ())
}

fn inspect_raw_input(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<usize, RegistryCodecError> {
    let mut max_depth = 0;
    inspect_raw_value(input, field, context, &mut max_depth, 0)?;
    Ok(max_depth)
}

// Child paths are needed only when a budget check fails.
fn inspect_raw_value(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    max_depth: &mut usize,
    depth: usize,
) -> Result<(), RegistryCodecError> {
    context.charge_node(field)?;
    let array_len = match input {
        RegistryElementValueRef::Nbt(bbb_protocol::NbtSummaryValue::ByteArray(values)) => {
            Some(values.len())
        }
        RegistryElementValueRef::Nbt(bbb_protocol::NbtSummaryValue::IntArray(values)) => {
            Some(values.len())
        }
        RegistryElementValueRef::Nbt(bbb_protocol::NbtSummaryValue::LongArray(values)) => {
            Some(values.len())
        }
        _ => None,
    };
    if let Some(len) = array_len {
        *max_depth = (*max_depth).max(depth + 1);
        context.check_collection(field, len)?;
        context.charge_nodes(field, len)?;
        return Ok(());
    }
    if let Some(len) = input.map_len() {
        *max_depth = (*max_depth).max(depth + 1);
        context.check_collection(field, len)?;
        for index in 0..len {
            let (name, value) = input
                .map_entry_java_string_ref(index)
                .expect("map length agrees with entries");
            context
                .with_nested(field, |context| {
                    inspect_raw_value(value, field, context, max_depth, depth + 1)
                })
                .map_err(|mut error| {
                    error
                        .field
                        .insert_str(field.len(), &format!(".{}", name.to_display_string()));
                    error
                })?;
        }
    } else if let Some(len) = input.list_len() {
        *max_depth = (*max_depth).max(depth + 1);
        context.check_collection(field, len)?;
        for index in 0..len {
            let value = input
                .list_element(index)
                .expect("list length agrees with elements");
            context
                .with_nested(field, |context| {
                    inspect_raw_value(value, field, context, max_depth, depth + 1)
                })
                .map_err(|mut error| {
                    error.field.insert_str(field.len(), &format!("[{index}]"));
                    error
                })?;
        }
    }
    Ok(())
}
