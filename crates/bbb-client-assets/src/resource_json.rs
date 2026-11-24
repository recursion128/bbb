use std::{collections::HashMap, fmt};

use anyhow::Context;
use bbb_protocol::java_number::MAX_GSON_STRICT_JSON_NUMBER_BYTES;
use bbb_protocol::{NbtSummaryDecodeLimits, NbtSummaryEntry, NbtSummaryValue};
use serde::{
    de::{Error as _, MapAccess, Visitor},
    Deserialize, Deserializer,
};
use serde_json::{value::RawValue, Number};

pub(crate) const MAX_RESOURCE_JSON_DEPTH: usize = 127;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceJsonString(Vec<u8>);

impl ResourceJsonString {
    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.0).ok()
    }

    pub fn from_text(value: &str) -> Self {
        Self(value.as_bytes().to_vec())
    }

    /// The Java `String` the reader would have built: one unit per BMP code
    /// point, a surrogate pair for anything above it, and an unpaired surrogate
    /// kept as the single unit it was written as.
    pub fn utf16_units(&self) -> Vec<u16> {
        self.utf16_units_iter().collect()
    }

    pub fn eq_utf16_units(&self, expected: &[u16]) -> bool {
        self.utf16_units_iter().eq(expected.iter().copied())
    }

    fn utf16_units_iter(&self) -> impl Iterator<Item = u16> + '_ {
        self.codepoints().flat_map(|point| {
            let mut pair = [0; 2];
            let len = if point <= 0xffff {
                pair[0] = point as u16;
                1
            } else {
                let point = point - 0x10000;
                pair[0] = 0xd800 + (point >> 10) as u16;
                pair[1] = 0xdc00 + (point & 0x3ff) as u16;
                2
            };
            pair.into_iter().take(len)
        })
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn starts_with(&self, prefix: &str) -> bool {
        self.0.starts_with(prefix.as_bytes())
    }

    pub fn codepoints(&self) -> impl Iterator<Item = u32> + '_ {
        let mut bytes = self.0.iter().copied();
        std::iter::from_fn(move || {
            let first = bytes.next()?;
            let (mut point, remaining) = match first {
                0..=0x7f => (u32::from(first), 0),
                0xc0..=0xdf => (u32::from(first & 0x1f), 1),
                0xe0..=0xef => (u32::from(first & 0x0f), 2),
                _ => (u32::from(first & 0x07), 3),
            };
            for _ in 0..remaining {
                point = (point << 6) | u32::from(bytes.next()? & 0x3f);
            }
            Some(point)
        })
    }
}

impl From<&str> for ResourceJsonString {
    fn from(value: &str) -> Self {
        Self::from_text(value)
    }
}

/// A `&str` is its own WTF-8 encoding, so comparing bytes answers exactly
/// whether the raw string is that literal.
impl PartialEq<str> for ResourceJsonString {
    fn eq(&self, other: &str) -> bool {
        self.0 == other.as_bytes()
    }
}

impl PartialEq<&str> for ResourceJsonString {
    fn eq(&self, other: &&str) -> bool {
        self.0 == other.as_bytes()
    }
}

impl PartialEq<String> for ResourceJsonString {
    fn eq(&self, other: &String) -> bool {
        self.0 == other.as_bytes()
    }
}

struct RawString(ResourceJsonString);

impl<'de> Deserialize<'de> for RawString {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct StringVisitor;
        impl Visitor<'_> for StringVisitor {
            type Value = ResourceJsonString;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON string")
            }

            fn visit_bytes<E: serde::de::Error>(self, bytes: &[u8]) -> Result<Self::Value, E> {
                Ok(ResourceJsonString(bytes.to_vec()))
            }
        }
        // The bytes visitor preserves escaped surrogates and rejects byte arrays.
        deserializer.deserialize_bytes(StringVisitor).map(Self)
    }
}

/// The object body of a [`ResourceJsonValue`]. Vanilla's `JsonObject` keeps
/// insertion order and a key is a Java `String`, so entries stay a sequence of
/// raw keys rather than a sorted map.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResourceJsonObject(Vec<(ResourceJsonString, ResourceJsonValue)>);

impl ResourceJsonObject {
    pub fn get(&self, key: &str) -> Option<&ResourceJsonValue> {
        self.0
            .iter()
            .find(|(candidate, _)| candidate.0 == key.as_bytes())
            .map(|(_, value)| value)
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, (ResourceJsonString, ResourceJsonValue)> {
        self.0.iter()
    }

    pub fn keys(&self) -> impl Iterator<Item = &ResourceJsonString> {
        self.0.iter().map(|(key, _)| key)
    }
}

impl std::ops::Index<&str> for ResourceJsonObject {
    type Output = ResourceJsonValue;

    fn index(&self, key: &str) -> &Self::Output {
        self.get(key).expect("resource JSON object key")
    }
}

impl std::ops::Index<usize> for ResourceJsonObject {
    type Output = (ResourceJsonString, ResourceJsonValue);

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<'a> IntoIterator for &'a ResourceJsonObject {
    type Item = &'a (ResourceJsonString, ResourceJsonValue);
    type IntoIter = std::slice::Iter<'a, (ResourceJsonString, ResourceJsonValue)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl FromIterator<(ResourceJsonString, ResourceJsonValue)> for ResourceJsonObject {
    fn from_iter<I: IntoIterator<Item = (ResourceJsonString, ResourceJsonValue)>>(
        entries: I,
    ) -> Self {
        Self(entries.into_iter().collect())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceJsonValue {
    Null,
    Bool(bool),
    // Parsed numbers keep their RawValue spelling.
    Number(String),
    String(ResourceJsonString),
    Array(Vec<ResourceJsonValue>),
    Object(ResourceJsonObject),
}

impl ResourceJsonValue {
    pub fn from_slice(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        // deserialize_bytes also accepts invalid raw UTF-8; resource JSON does not.
        std::str::from_utf8(bytes).map_err(serde_json::Error::custom)?;
        let raw = serde_json::from_slice::<&RawValue>(bytes)?;
        Self::from_raw(raw, 0)
    }

    fn from_raw(raw: &RawValue, depth: usize) -> Result<Self, serde_json::Error> {
        let source = raw.get();
        let value = match source.as_bytes()[0] {
            b'"' => Self::String(serde_json::from_str::<RawString>(source)?.0),
            b'[' | b'{' if depth >= MAX_RESOURCE_JSON_DEPTH => {
                return Err(serde_json::Error::custom(
                    "resource JSON recursion limit exceeded",
                ));
            }
            b'[' => {
                let values: Vec<&RawValue> = serde_json::from_str(source)?;
                Self::Array(
                    values
                        .into_iter()
                        .map(|value| Self::from_raw(value, depth + 1))
                        .collect::<Result<_, _>>()?,
                )
            }
            b'{' => {
                let RawObject(entries) = serde_json::from_str(source)?;
                let mut positions: HashMap<ResourceJsonString, usize> = HashMap::new();
                let mut values: Vec<(ResourceJsonString, Self)> = Vec::new();
                for (key, raw) in entries {
                    let value = Self::from_raw(raw, depth + 1)?;
                    if let Some(&index) = positions.get(&key) {
                        values[index].1 = value;
                    } else {
                        positions.insert(key.clone(), values.len());
                        values.push((key, value));
                    }
                }
                Self::Object(ResourceJsonObject(values))
            }
            b'n' => Self::Null,
            b't' | b'f' => Self::Bool(serde_json::from_str(source)?),
            _ => {
                if source.len() > MAX_GSON_STRICT_JSON_NUMBER_BYTES {
                    return Err(serde_json::Error::custom(
                        "resource JSON number token exceeds Gson's strict reader limit",
                    ));
                }
                Self::Number(source.to_owned())
            }
        };
        Ok(value)
    }

    /// Builds an object out of decoded entries, for an owner that regroups the
    /// fields it read rather than re-reading the source.
    pub fn from_entries(entries: Vec<(ResourceJsonString, Self)>) -> Self {
        Self::Object(ResourceJsonObject(entries))
    }

    pub fn from_text(value: &str) -> Self {
        Self::String(ResourceJsonString::from_text(value))
    }

    pub fn get(&self, key: &str) -> Option<&Self> {
        self.as_object()?.get(key)
    }

    pub fn as_array(&self) -> Option<&Vec<Self>> {
        match self {
            Self::Array(values) => Some(values),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&ResourceJsonObject> {
        match self {
            Self::Object(entries) => Some(entries),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&ResourceJsonString> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        self.as_string()?.as_str()
    }

    pub fn is_string(&self) -> bool {
        self.as_string().is_some()
    }

    pub fn is_boolean(&self) -> bool {
        self.as_bool().is_some()
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(value) => value.parse::<Number>().ok()?.as_i64(),
            _ => None,
        }
    }

    pub fn codec_i32(&self) -> Option<i32> {
        match self {
            Self::Number(value) => bbb_protocol::java_number::json_number_i32(value),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Number(value) => value.parse::<Number>().ok()?.as_u64(),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(value) => value.parse::<Number>().ok()?.as_f64(),
            _ => None,
        }
    }

    pub(crate) fn to_registry_json(&self) -> bbb_registry_codec::registry_codec::RegistryJsonValue {
        use bbb_registry_codec::registry_codec::RegistryJsonValue as Value;

        match self {
            Self::Null => Value::Null,
            Self::Bool(value) => Value::Bool(*value),
            Self::Number(value) => Value::Number(value.clone()),
            Self::String(value) => Value::String(value.utf16_units().into()),
            Self::Array(values) => {
                Value::Array(values.iter().map(Self::to_registry_json).collect())
            }
            Self::Object(entries) => Value::Object(
                entries
                    .iter()
                    .map(|(name, value)| (name.utf16_units().into(), value.to_registry_json()))
                    .collect(),
            ),
        }
    }

    /// The tag `JsonOps` builds for this value, mirroring
    /// `bbb_protocol::json_value_to_nbt_summary` over the raw reader: a string
    /// keeps the Java units it was written with, so an escaped lone surrogate
    /// reaches the tag instead of collapsing to a replacement character.
    pub fn to_nbt_summary(&self) -> anyhow::Result<NbtSummaryValue> {
        self.to_nbt_summary_at_depth(0)
    }

    fn to_nbt_summary_at_depth(&self, depth: usize) -> anyhow::Result<NbtSummaryValue> {
        let limits = NbtSummaryDecodeLimits::default();
        if depth > limits.max_depth {
            anyhow::bail!("json-to-nbt value exceeded max depth");
        }
        Ok(match self {
            Self::Null => anyhow::bail!("null has no NBT value representation"),
            Self::Bool(value) => NbtSummaryValue::Byte(i8::from(*value)),
            Self::Number(value) => number_to_nbt_summary(
                &value
                    .parse::<Number>()
                    .context("json number is not representable")?,
            )?,
            Self::String(value) => NbtSummaryValue::string_from_utf16(value.utf16_units()),
            Self::Array(values) => {
                if values.len() > limits.max_list_items {
                    anyhow::bail!("json list exceeds the NBT list bound");
                }
                NbtSummaryValue::List(
                    values
                        .iter()
                        .map(|value| value.to_nbt_summary_at_depth(depth + 1))
                        .collect::<anyhow::Result<_>>()?,
                )
            }
            Self::Object(entries) => {
                if entries.len() > limits.max_compound_entries {
                    anyhow::bail!("json object exceeds the NBT compound bound");
                }
                NbtSummaryValue::Compound(
                    entries
                        .iter()
                        .map(|(name, value)| {
                            Ok(NbtSummaryEntry {
                                name: name
                                    .as_str()
                                    .context("json object key is not representable as a tag name")?
                                    .into(),
                                value: value.to_nbt_summary_at_depth(depth + 1)?,
                            })
                        })
                        .collect::<anyhow::Result<_>>()?,
                )
            }
        })
    }

    pub(crate) fn tree_size(&self) -> anyhow::Result<(u64, u64)> {
        let mut nodes = 0_u64;
        let mut string_bytes = 0_u64;
        let mut pending = vec![self];
        while let Some(value) = pending.pop() {
            nodes = nodes
                .checked_add(1)
                .context("JSON value node count overflow")?;
            match value {
                ResourceJsonValue::Number(value) => {
                    string_bytes = string_bytes
                        .checked_add(
                            u64::try_from(value.len()).context("JSON number size overflow")?,
                        )
                        .context("JSON decoded string byte count overflow")?;
                }
                ResourceJsonValue::String(value) => {
                    string_bytes = string_bytes
                        .checked_add(
                            u64::try_from(value.0.len()).context("JSON string size overflow")?,
                        )
                        .context("JSON decoded string byte count overflow")?;
                }
                ResourceJsonValue::Array(values) => pending.extend(values),
                ResourceJsonValue::Object(values) => {
                    for (key, value) in values {
                        string_bytes = string_bytes
                            .checked_add(
                                u64::try_from(key.0.len()).context("JSON key size overflow")?,
                            )
                            .context("JSON decoded key byte count overflow")?;
                        pending.push(value);
                    }
                }
                _ => {}
            }
        }
        Ok((nodes, string_bytes))
    }
}

/// A `serde_json::Value` has already normalized number spellings and strings.
impl From<&serde_json::Value> for ResourceJsonValue {
    fn from(value: &serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::Bool(value) => Self::Bool(*value),
            serde_json::Value::Number(value) => Self::Number(value.to_string()),
            serde_json::Value::String(value) => Self::String(ResourceJsonString::from_text(value)),
            serde_json::Value::Array(values) => {
                Self::Array(values.iter().map(Self::from).collect())
            }
            serde_json::Value::Object(entries) => Self::Object(
                entries
                    .iter()
                    .map(|(name, value)| (ResourceJsonString::from_text(name), Self::from(value)))
                    .collect(),
            ),
        }
    }
}

impl From<serde_json::Value> for ResourceJsonValue {
    fn from(value: serde_json::Value) -> Self {
        Self::from(&value)
    }
}

fn number_to_nbt_summary(value: &Number) -> anyhow::Result<NbtSummaryValue> {
    if let Some(value) = value.as_i64() {
        return Ok(match i32::try_from(value) {
            Ok(value) => NbtSummaryValue::Int(value),
            Err(_) => NbtSummaryValue::Long(value),
        });
    }
    if let Some(value) = value.as_u64() {
        let value = i64::try_from(value).context("json integer exceeds the signed NBT domain")?;
        return Ok(match i32::try_from(value) {
            Ok(value) => NbtSummaryValue::Int(value),
            Err(_) => NbtSummaryValue::Long(value),
        });
    }
    let value = value
        .as_f64()
        .context("json number has no finite f64 representation")?;
    Ok(NbtSummaryValue::Double(value.to_bits()))
}

struct RawObject<'a>(Vec<(ResourceJsonString, &'a RawValue)>);

impl<'de> Deserialize<'de> for RawObject<'de> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ObjectVisitor;
        impl<'de> Visitor<'de> for ObjectVisitor {
            type Value = RawObject<'de>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON object")
            }

            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
                let mut entries = Vec::new();
                while let Some((RawString(key), value)) = map.next_entry()? {
                    entries.push((key, value));
                }
                Ok(RawObject(entries))
            }
        }
        deserializer.deserialize_map(ObjectVisitor)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod utf16_comparison_tests {
    use super::ResourceJsonValue;

    #[test]
    fn escaped_surrogates_compare_as_raw_units_without_replacement() {
        for (json, units) in [
            (br#""\uD800""#.as_slice(), vec![0xd800]),
            (br#""\uD801""#.as_slice(), vec![0xd801]),
            (br#""\uDC00""#.as_slice(), vec![0xdc00]),
            (br#""\uFFFD""#.as_slice(), vec![0xfffd]),
            (br#""?""#.as_slice(), vec![0x003f]),
            (br#""\uD83D\uDE00""#.as_slice(), vec![0xd83d, 0xde00]),
        ] {
            let parsed = ResourceJsonValue::from_slice(json).unwrap();
            let string = parsed.as_string().unwrap();
            assert_eq!(string.utf16_units(), units);
            assert!(string.eq_utf16_units(&units));
            assert!(!string.eq_utf16_units(&[0xfffd, 0x003f]));
        }
    }
}

#[cfg(test)]
mod strict_json_number_tests {
    use super::ResourceJsonValue;
    use bbb_protocol::java_number::MAX_GSON_STRICT_JSON_NUMBER_BYTES;

    #[test]
    fn strict_resource_json_number_limit_applies_in_unknown_fields() {
        let accepted = "1".repeat(MAX_GSON_STRICT_JSON_NUMBER_BYTES);
        let rejected = "1".repeat(MAX_GSON_STRICT_JSON_NUMBER_BYTES + 1);
        assert!(ResourceJsonValue::from_slice(accepted.as_bytes()).is_ok());
        assert!(ResourceJsonValue::from_slice(rejected.as_bytes()).is_err());
        assert!(ResourceJsonValue::from_slice(
            format!(r#"{{"unknown":{rejected},"potion":"healing"}}"#).as_bytes()
        )
        .is_err());
        assert!(
            ResourceJsonValue::from_slice(format!(r#"{{"unknown":"{rejected}"}}"#).as_bytes())
                .is_ok()
        );
        let exponent = ResourceJsonValue::from_slice(br#"{"number":1e2,"quoted":"1e2"}"#).unwrap();
        assert_eq!(exponent.get("number").unwrap().codec_i32(), Some(100));
        assert_eq!(exponent.get("quoted").unwrap().codec_i32(), None);
    }
}
