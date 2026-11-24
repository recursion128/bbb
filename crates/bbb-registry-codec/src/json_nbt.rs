//! Datapack JSON as the NBT tree `JsonOps.convertTo(NbtOps)` builds from it,
//! so a JSON-sourced value is read by the same function as a saved one.
//!
//! DFU `JsonOps.convertTo` gives a number the narrowest of byte/short/int/long
//! that holds it exactly, else float or double; a boolean becomes a `ByteTag`,
//! an array a `ListTag`, an object a `CompoundTag` keeping the last of a
//! repeated key. A `null` member is absent, matching
//! `JsonOps.getMap`.

use std::borrow::Cow;

use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::registry_codec::{RegistryElementValueRef, RegistryJsonValue};

const MAX_DEPTH: usize = 512;

/// The NBT view of a registry codec input: NBT is borrowed, JSON is converted.
pub(crate) fn element_value_to_nbt(
    input: RegistryElementValueRef<'_>,
) -> Result<Cow<'_, NbtSummaryValue>, String> {
    Ok(match input {
        RegistryElementValueRef::Nbt(value) => Cow::Borrowed(value),
        RegistryElementValueRef::Json(value) => Cow::Owned(json_to_nbt(value)?),
        RegistryElementValueRef::NbtByte(value) => Cow::Owned(NbtSummaryValue::Byte(value)),
        RegistryElementValueRef::NbtInt(value) => Cow::Owned(NbtSummaryValue::Int(value)),
        RegistryElementValueRef::NbtLong(value) => Cow::Owned(NbtSummaryValue::Long(value)),
    })
}

/// Converts one JSON value; a top-level `null` has no tag to become.
pub fn json_to_nbt(value: &RegistryJsonValue) -> Result<NbtSummaryValue, String> {
    convert(value, 0)?.ok_or_else(|| "null has no NBT value".to_string())
}

fn convert(value: &RegistryJsonValue, depth: usize) -> Result<Option<NbtSummaryValue>, String> {
    if depth > MAX_DEPTH {
        return Err(format!("JSON nesting exceeds {MAX_DEPTH}"));
    }
    Ok(Some(match value {
        RegistryJsonValue::Null => return Ok(None),
        RegistryJsonValue::Bool(value) => NbtSummaryValue::Byte(i8::from(*value)),
        RegistryJsonValue::Number(lexeme) => {
            bbb_protocol::nbt_summary::json_number_to_nbtops(lexeme)
                .map_err(|_| format!("JSON number {lexeme} has no JsonOps value"))?
        }
        RegistryJsonValue::String(value) => NbtSummaryValue::String(value.clone()),
        RegistryJsonValue::Array(values) => NbtSummaryValue::List(
            values
                .iter()
                .map(|value| convert(value, depth + 1))
                .filter_map(Result::transpose)
                .collect::<Result<_, _>>()?,
        ),
        RegistryJsonValue::Object(entries) => {
            let fields = if entries.len() > 32 {
                let mut slots: Vec<Option<NbtSummaryEntry>> = Vec::with_capacity(entries.len());
                let mut index: std::collections::HashMap<&bbb_protocol::JavaString, usize> =
                    std::collections::HashMap::with_capacity(entries.len());
                for (name, value) in entries {
                    let Some(value) = convert(value, depth + 1)? else {
                        if let Some(pos) = index.remove(name) {
                            slots[pos] = None;
                        }
                        continue;
                    };
                    if let Some(&pos) = index.get(name) {
                        if let Some(entry) = slots[pos].as_mut() {
                            entry.value = value;
                        }
                    } else {
                        let pos = slots.len();
                        index.insert(name, pos);
                        slots.push(Some(NbtSummaryEntry {
                            name: name.clone(),
                            value,
                        }));
                    }
                }
                slots.into_iter().flatten().collect()
            } else {
                let mut fields: Vec<NbtSummaryEntry> = Vec::with_capacity(entries.len());
                for (name, value) in entries {
                    let Some(value) = convert(value, depth + 1)? else {
                        fields.retain(|field| &field.name != name);
                        continue;
                    };
                    match fields.iter_mut().find(|field| &field.name == name) {
                        Some(field) => field.value = value,
                        None => fields.push(NbtSummaryEntry {
                            name: name.clone(),
                            value,
                        }),
                    }
                }
                fields
            };
            NbtSummaryValue::Compound(fields)
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_converts_through_nbt_ops_shapes() {
        let json = RegistryJsonValue::Object(vec![
            (
                "id".into(),
                RegistryJsonValue::String("minecraft:stone".into()),
            ),
            ("count".into(), RegistryJsonValue::Number("3".into())),
            ("flag".into(), RegistryJsonValue::Bool(true)),
            ("gone".into(), RegistryJsonValue::Null),
            ("removed".into(), RegistryJsonValue::Number("7".into())),
            ("removed".into(), RegistryJsonValue::Null),
            ("count".into(), RegistryJsonValue::Number("4".into())),
            (
                "list".into(),
                RegistryJsonValue::Array(vec![RegistryJsonValue::Number("1.5".into())]),
            ),
        ]);
        let entry = |name: &str, value| NbtSummaryEntry {
            name: name.into(),
            value,
        };
        assert_eq!(
            json_to_nbt(&json).unwrap(),
            NbtSummaryValue::Compound(vec![
                entry("id", NbtSummaryValue::String("minecraft:stone".into())),
                entry("count", NbtSummaryValue::Byte(4)),
                entry("flag", NbtSummaryValue::Byte(1)),
                entry(
                    "list",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Float(1.5f32.to_bits())])
                ),
            ])
        );
        assert!(json_to_nbt(&RegistryJsonValue::Null).is_err());
    }
}
