//! Shared field readers for Java 26.1 persistence schemas.

pub use bbb_protocol::nbt_summary::{
    codec_float_value as float_value, codec_int_value as int_value, codec_long_value as long_value,
    compound_field as field,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::save::{NamedCompound, SaveError, SaveResult, JAVA_26_1_DATA_VERSION};

pub fn invalid_schema(path: &str, detail: impl Into<String>) -> SaveError {
    SaveError::InvalidNbt {
        path: None,
        detail: format!("{path}: {}", detail.into()),
    }
}

/// Vanilla `NbtIo` ignores the root tag name.
pub fn require_java_26_1_root<'a>(
    document: &'a NamedCompound,
    schema: &str,
) -> SaveResult<&'a [NbtSummaryEntry]> {
    let entries = document.entries();
    let version_path = format!("{schema}.DataVersion");
    let actual = expect_int(
        required_field(entries, "DataVersion", schema)?,
        &version_path,
    )?;
    if actual != JAVA_26_1_DATA_VERSION {
        return Err(SaveError::UnsupportedDataVersion {
            expected: JAVA_26_1_DATA_VERSION,
            actual,
        });
    }
    Ok(entries)
}

pub fn required_field<'a>(
    entries: &'a [NbtSummaryEntry],
    name: &str,
    parent_path: &str,
) -> SaveResult<&'a NbtSummaryValue> {
    field(entries, name).ok_or_else(|| {
        invalid_schema(
            &format!("{parent_path}.{name}"),
            "required field is missing",
        )
    })
}

/// The compound under `name`, or empty when it is missing or another tag.
pub fn compound_or_empty<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> &'a [NbtSummaryEntry] {
    match field(entries, name) {
        Some(NbtSummaryValue::Compound(entries)) => entries,
        _ => &[],
    }
}

/// The list under `name`, or empty when it is missing or another tag.
pub fn list_or_empty<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> &'a [NbtSummaryValue] {
    match field(entries, name) {
        Some(NbtSummaryValue::List(values)) => values,
        _ => &[],
    }
}

/// A fixed-size `Codec.INT_STREAM` (`BlockPos`, `UUIDUtil`): an int array or a
/// list of numbers.
pub use bbb_protocol::nbt_summary::codec_int_array as int_array;

/// `UUIDUtil.CODEC` (`UUIDUtil.java:23-26`).
pub use bbb_protocol::nbt_summary::codec_uuid as uuid_value;

pub use bbb_protocol::nbt_summary::uuid_nbt;

/// `Codec.BOOL` decodes any number, non-zero meaning true.
pub fn bool_value(value: &NbtSummaryValue) -> Option<bool> {
    long_value(value).map(|value| value != 0)
}

pub fn int_or(entries: &[NbtSummaryEntry], name: &str, default: i32) -> i32 {
    field(entries, name).and_then(int_value).unwrap_or(default)
}

pub fn long_or(entries: &[NbtSummaryEntry], name: &str, default: i64) -> i64 {
    field(entries, name).and_then(long_value).unwrap_or(default)
}

pub fn float_or(entries: &[NbtSummaryEntry], name: &str, default: f32) -> f32 {
    field(entries, name)
        .and_then(float_value)
        .unwrap_or(default)
}

pub fn bool_or(entries: &[NbtSummaryEntry], name: &str, default: bool) -> bool {
    field(entries, name).and_then(bool_value).unwrap_or(default)
}

pub fn expect_compound<'a>(
    value: &'a NbtSummaryValue,
    path: &str,
) -> SaveResult<&'a [NbtSummaryEntry]> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(invalid_schema(path, "must be a compound"));
    };
    Ok(entries)
}

pub fn expect_int(value: &NbtSummaryValue, path: &str) -> SaveResult<i32> {
    let NbtSummaryValue::Int(value) = value else {
        return Err(invalid_schema(path, "must be an int"));
    };
    Ok(*value)
}

pub fn expect_resource_id(value: &NbtSummaryValue, path: &str) -> SaveResult<String> {
    let NbtSummaryValue::String(raw) = value else {
        return Err(invalid_schema(path, "must be a string"));
    };
    bbb_protocol::normalize_resource_location(
        raw.try_to_string()
            .map_err(|_| invalid_schema(path, "invalid resource id"))?,
    )
    .map_err(|error| invalid_schema(path, format!("invalid resource id: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_accepts_only_data_version_4786_and_ignores_the_root_name() {
        let root = |version| {
            NamedCompound::new(
                "named",
                vec![NbtSummaryEntry {
                    name: "DataVersion".into(),
                    value: NbtSummaryValue::Int(version),
                }],
            )
        };
        assert_eq!(
            require_java_26_1_root(&root(JAVA_26_1_DATA_VERSION), "chunk")
                .unwrap()
                .len(),
            1
        );
        assert!(require_java_26_1_root(&root(JAVA_26_1_DATA_VERSION + 1), "chunk").is_err());
    }
}
