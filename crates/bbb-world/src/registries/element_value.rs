use bbb_protocol::{decode_nbt_summary, NbtSummaryEntry, NbtSummaryValue};

const MAX_REGISTRY_ELEMENT_DEPTH: usize = 64;
const MAX_REGISTRY_ELEMENT_ITEMS: usize = 4096;
const MAX_REGISTRY_ELEMENT_JSON_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryElementSource {
    NetworkNbt,
    KnownPackJson,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryElementData {
    pub source: RegistryElementSource,
    pub value: NbtSummaryValue,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RegistryElementDecodeError {
    #[error("invalid network registry element: {0}")]
    Network(String),
    #[error("known-pack registry element is {actual} bytes, maximum is {maximum}")]
    JsonTooLarge { actual: usize, maximum: usize },
    #[error("invalid known-pack registry JSON: {0}")]
    Json(String),
    #[error("known-pack registry JSON root must not be null")]
    NullRoot,
    #[error("known-pack registry JSON exceeded depth {maximum}")]
    JsonDepth { maximum: usize },
    #[error("known-pack registry JSON collection has {actual} items, maximum is {maximum}")]
    JsonCollectionTooLarge { actual: usize, maximum: usize },
    #[error("known-pack registry JSON integer {0} is outside the signed 64-bit value domain")]
    JsonIntegerOutOfRange(u64),
}

pub(super) fn decode_network_element(
    raw_data: &[u8],
) -> Result<RegistryElementData, RegistryElementDecodeError> {
    let mut value = decode_nbt_summary(raw_data)
        .map_err(|error| RegistryElementDecodeError::Network(error.to_string()))?
        .ok_or_else(|| {
            RegistryElementDecodeError::Network(
                "registry element root must not be TAG_End".to_string(),
            )
        })?;
    canonicalize_element_value(&mut value);
    Ok(RegistryElementData {
        source: RegistryElementSource::NetworkNbt,
        value,
    })
}

pub(super) fn decode_known_pack_element(
    json: &[u8],
) -> Result<RegistryElementData, RegistryElementDecodeError> {
    if json.len() > MAX_REGISTRY_ELEMENT_JSON_BYTES {
        return Err(RegistryElementDecodeError::JsonTooLarge {
            actual: json.len(),
            maximum: MAX_REGISTRY_ELEMENT_JSON_BYTES,
        });
    }
    let value: serde_json::Value = serde_json::from_slice(json)
        .map_err(|error| RegistryElementDecodeError::Json(error.to_string()))?;
    if value.is_null() {
        return Err(RegistryElementDecodeError::NullRoot);
    }
    let mut value = json_to_element_value(value, 0)?;
    canonicalize_element_value(&mut value);
    Ok(RegistryElementData {
        source: RegistryElementSource::KnownPackJson,
        value,
    })
}

fn json_to_element_value(
    value: serde_json::Value,
    depth: usize,
) -> Result<NbtSummaryValue, RegistryElementDecodeError> {
    match value {
        serde_json::Value::Null => Err(RegistryElementDecodeError::Json(
            "null has no NBT value representation".to_string(),
        )),
        serde_json::Value::Bool(value) => Ok(NbtSummaryValue::Byte(i8::from(value))),
        serde_json::Value::Number(value) => json_number_to_element_value(value),
        serde_json::Value::String(value) => Ok(NbtSummaryValue::String(value)),
        serde_json::Value::Array(values) => {
            let nested_depth = enter_json_container(depth)?;
            check_collection_len(values.len())?;
            values
                .into_iter()
                .map(|value| json_to_element_value(value, nested_depth))
                .collect::<Result<Vec<_>, _>>()
                .map(NbtSummaryValue::List)
        }
        serde_json::Value::Object(entries) => {
            let nested_depth = enter_json_container(depth)?;
            check_collection_len(entries.len())?;
            entries
                .into_iter()
                .map(|(name, value)| {
                    Ok(NbtSummaryEntry {
                        name,
                        value: json_to_element_value(value, nested_depth)?,
                    })
                })
                .collect::<Result<Vec<_>, _>>()
                .map(NbtSummaryValue::Compound)
        }
    }
}

fn json_number_to_element_value(
    value: serde_json::Number,
) -> Result<NbtSummaryValue, RegistryElementDecodeError> {
    if let Some(value) = value.as_i64() {
        return Ok(match i32::try_from(value) {
            Ok(value) => NbtSummaryValue::Int(value),
            Err(_) => NbtSummaryValue::Long(value),
        });
    }
    if let Some(value) = value.as_u64() {
        let value = i64::try_from(value)
            .map_err(|_| RegistryElementDecodeError::JsonIntegerOutOfRange(value))?;
        return Ok(match i32::try_from(value) {
            Ok(value) => NbtSummaryValue::Int(value),
            Err(_) => NbtSummaryValue::Long(value),
        });
    }
    let value = value.as_f64().ok_or_else(|| {
        RegistryElementDecodeError::Json("number has no finite f64 representation".to_string())
    })?;
    Ok(NbtSummaryValue::Double(value.to_bits()))
}

fn enter_json_container(depth: usize) -> Result<usize, RegistryElementDecodeError> {
    if depth >= MAX_REGISTRY_ELEMENT_DEPTH {
        return Err(RegistryElementDecodeError::JsonDepth {
            maximum: MAX_REGISTRY_ELEMENT_DEPTH,
        });
    }
    Ok(depth + 1)
}

fn check_collection_len(len: usize) -> Result<(), RegistryElementDecodeError> {
    if len > MAX_REGISTRY_ELEMENT_ITEMS {
        return Err(RegistryElementDecodeError::JsonCollectionTooLarge {
            actual: len,
            maximum: MAX_REGISTRY_ELEMENT_ITEMS,
        });
    }
    Ok(())
}

fn canonicalize_element_value(value: &mut NbtSummaryValue) {
    match value {
        NbtSummaryValue::Byte(number) => {
            *value = NbtSummaryValue::Long(i64::from(*number));
        }
        NbtSummaryValue::Short(number) => {
            *value = NbtSummaryValue::Long(i64::from(*number));
        }
        NbtSummaryValue::Int(number) => {
            *value = NbtSummaryValue::Long(i64::from(*number));
        }
        NbtSummaryValue::Float(bits) => {
            *value = NbtSummaryValue::Double(f64::from(f32::from_bits(*bits)).to_bits());
        }
        NbtSummaryValue::ByteArray(numbers) => {
            *value = NbtSummaryValue::List(
                std::mem::take(numbers)
                    .into_iter()
                    .map(|number| NbtSummaryValue::Long(i64::from(number)))
                    .collect(),
            );
        }
        NbtSummaryValue::IntArray(numbers) => {
            *value = NbtSummaryValue::List(
                std::mem::take(numbers)
                    .into_iter()
                    .map(|number| NbtSummaryValue::Long(i64::from(number)))
                    .collect(),
            );
        }
        NbtSummaryValue::LongArray(numbers) => {
            *value = NbtSummaryValue::List(
                std::mem::take(numbers)
                    .into_iter()
                    .map(NbtSummaryValue::Long)
                    .collect(),
            );
        }
        NbtSummaryValue::List(values) => {
            for value in values {
                canonicalize_element_value(value);
            }
        }
        NbtSummaryValue::Compound(entries) => {
            for entry in entries.iter_mut() {
                canonicalize_element_value(&mut entry.value);
            }
            entries.sort_by(|left, right| left.name.cmp(&right.name));
        }
        NbtSummaryValue::Long(_) | NbtSummaryValue::Double(_) | NbtSummaryValue::String(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_nbt_and_known_pack_json_share_one_value_shape() {
        let network = decode_network_element(&[
            10, 8, 0, 2, b'i', b'd', 0, 4, b'c', b'o', b'r', b'e', 1, 0, 7, b'e', b'n', b'a', b'b',
            b'l', b'e', b'd', 1, 3, 0, 5, b'c', b'o', b'u', b'n', b't', 0, 0, 0, 42, 5, 0, 5, b'r',
            b'a', b't', b'i', b'o', 0x3f, 0xc0, 0, 0, 0,
        ])
        .unwrap();
        let known_pack =
            decode_known_pack_element(br#"{"id":"core","enabled":true,"count":42,"ratio":1.5}"#)
                .unwrap();

        assert_eq!(network.source, RegistryElementSource::NetworkNbt);
        assert_eq!(known_pack.source, RegistryElementSource::KnownPackJson);
        assert_eq!(network.value, known_pack.value);
    }

    #[test]
    fn known_pack_json_rejects_null_root_and_oversized_collections() {
        assert_eq!(
            decode_known_pack_element(b"null"),
            Err(RegistryElementDecodeError::NullRoot)
        );
        let value = serde_json::Value::Array(vec![
            serde_json::Value::Bool(false);
            MAX_REGISTRY_ELEMENT_ITEMS + 1
        ]);
        assert_eq!(
            json_to_element_value(value, 0),
            Err(RegistryElementDecodeError::JsonCollectionTooLarge {
                actual: MAX_REGISTRY_ELEMENT_ITEMS + 1,
                maximum: MAX_REGISTRY_ELEMENT_ITEMS,
            })
        );
    }

    #[test]
    fn known_pack_json_rejects_values_beyond_the_nbt_numeric_domain() {
        assert_eq!(
            decode_known_pack_element(b"18446744073709551615"),
            Err(RegistryElementDecodeError::JsonIntegerOutOfRange(u64::MAX))
        );
    }

    #[test]
    fn known_pack_json_rejects_excessive_container_depth() {
        let mut value = serde_json::Value::Bool(true);
        for _ in 0..=MAX_REGISTRY_ELEMENT_DEPTH {
            value = serde_json::Value::Array(vec![value]);
        }
        assert_eq!(
            json_to_element_value(value, 0),
            Err(RegistryElementDecodeError::JsonDepth {
                maximum: MAX_REGISTRY_ELEMENT_DEPTH,
            })
        );
    }
}
