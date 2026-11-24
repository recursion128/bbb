use serde::Serialize;

use bbb_protocol::java_number::{
    json_number_i32, json_number_i64,
    MAX_GSON_STRICT_JSON_NUMBER_BYTES as MAX_REGISTRY_ELEMENT_JSON_NUMBER_BYTES,
};
use bbb_protocol::{
    decode_nbt_summary_with_limits,
    nbt_summary::{
        canonical_double_tag_bits, codec_double_value, codec_float_value, codec_int_value,
        codec_long_value, entry, NbtSummaryDecodeLimits,
    },
    NbtSummaryEntry, NbtSummaryValue, VANILLA_DEFAULT_NBT_QUOTA,
};

const MAX_REGISTRY_ELEMENT_DEPTH: usize = 64;
const MAX_REGISTRY_ELEMENT_ITEMS: usize = 4096;
const MAX_REGISTRY_ELEMENT_JSON_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryElementSource {
    NetworkNbt,
    ClientKnownPackJson,
    ServerDatapackJson,
}

/// A bounded JSON tree that retains every number's source lexeme.
///
/// `serde_json::Value` intentionally normalizes numbers. Vanilla's JsonOps
/// instead exposes Gson's lazily parsed source number to each primitive codec,
/// so registry elements keep this narrow source representation without
/// enabling serde_json's graph-wide `arbitrary_precision` behavior.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryJsonValue {
    Null,
    Bool(bool),
    Number(String),
    String(bbb_protocol::JavaString),
    Array(Vec<Self>),
    Object(Vec<(bbb_protocol::JavaString, Self)>),
}

impl Serialize for RegistryJsonValue {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Null => serializer.serialize_unit(),
            Self::Bool(value) => serializer.serialize_bool(*value),
            Self::Number(value) => {
                let raw: &serde_json::value::RawValue =
                    serde_json::from_str(value).map_err(serde::ser::Error::custom)?;
                if !matches!(
                    raw.get().as_bytes().first().copied(),
                    Some(b'-' | b'0'..=b'9')
                ) {
                    return Err(serde::ser::Error::custom("expected a JSON number"));
                }
                raw.serialize(serializer)
            }
            Self::String(value) => value.serialize(serializer),
            Self::Array(values) => values.serialize(serializer),
            Self::Object(entries) => bbb_protocol::serialize_java_string_map(
                entries.iter().map(|(key, value)| (key, value)),
                serializer,
            ),
        }
    }
}

impl RegistryJsonValue {
    pub fn get(&self, name: &str) -> Option<&Self> {
        self.as_object()?
            .iter()
            .rev()
            .find(|(entry_name, _)| entry_name == name)
            .map(|(_, value)| value)
    }

    pub fn as_object(&self) -> Option<&[(bbb_protocol::JavaString, Self)]> {
        match self {
            Self::Object(entries) => Some(entries),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Self]> {
        match self {
            Self::Array(values) => Some(values),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(value) => value.parse().ok(),
            _ => None,
        }
    }

    pub fn is_i64(&self) -> bool {
        matches!(self, Self::Number(value) if value.parse::<i64>().is_ok())
    }

    pub fn is_u64(&self) -> bool {
        matches!(self, Self::Number(value) if value.parse::<u64>().is_ok())
    }

    pub fn utf8_string(&self) -> Option<String> {
        match self {
            Self::String(value) => value.try_to_string().ok(),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(value) => value.parse().ok(),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Number(value) => value.parse().ok(),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }
    pub fn is_object(&self) -> bool {
        matches!(self, Self::Object(_))
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryElementData {
    pub source: RegistryElementSource,
    input: RegistryElementInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RegistryElementInput {
    Nbt(NbtSummaryValue),
    Json(RegistryJsonValue),
}

impl RegistryElementData {
    pub fn from_network_nbt(value: NbtSummaryValue) -> Self {
        Self {
            source: RegistryElementSource::NetworkNbt,
            input: RegistryElementInput::Nbt(value),
        }
    }

    #[cfg(test)]
    pub(crate) fn from_json_value(value: RegistryJsonValue, source: RegistryElementSource) -> Self {
        assert_ne!(source, RegistryElementSource::NetworkNbt);
        Self {
            source,
            input: RegistryElementInput::Json(value),
        }
    }

    pub fn json(&self) -> Option<&RegistryJsonValue> {
        match &self.input {
            RegistryElementInput::Json(value) => Some(value),
            _ => None,
        }
    }

    /// Legacy scalar projection with the source map's visible iteration order.
    pub fn source_order_nbt_projection(
        &self,
    ) -> Result<NbtSummaryValue, RegistryElementDecodeError> {
        match &self.input {
            RegistryElementInput::Nbt(value) => Ok(canonical_element_value(value)),
            RegistryElementInput::Json(json) => json_to_element_value(json, 0),
        }
    }

    /// Returns the DynamicOps input that exact registry codecs must read.
    ///
    /// JSON-backed elements retain their original tree so JSON null keeps the
    /// same context-sensitive meaning that it has in DFU `JsonOps`. Network
    /// elements always read the decoded NBT tree.
    pub fn root(&self) -> RegistryElementValueRef<'_> {
        match &self.input {
            RegistryElementInput::Nbt(value) => RegistryElementValueRef::Nbt(value),
            RegistryElementInput::Json(value) => RegistryElementValueRef::Json(value),
        }
    }
}

/// A borrowed registry codec input that preserves its source DynamicOps shape.
///
/// This is deliberately a narrow view rather than a second semantic value
/// model. Exact codecs use it to apply the same field, collection, and numeric
/// rules to network NBT and source JSON without adding JSON null to
/// `NbtSummaryValue`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryElementValueRef<'a> {
    Nbt(&'a NbtSummaryValue),
    Json(&'a RegistryJsonValue),
    /// Borrow-free scalar views for NBT primitive arrays. `NbtOps.getStream`
    /// exposes byte/int/long arrays as ordinary element streams, so holder and
    /// other list codecs must see their members without allocating a widened
    /// replacement tree.
    NbtByte(i8),
    NbtInt(i32),
    NbtLong(i64),
}

/// Keeps each raw key's first position and its last value.
pub(crate) fn collect_visible_map_entries<'a, V>(
    entries: impl Iterator<Item = (&'a bbb_protocol::JavaString, V)>,
    capacity: usize,
) -> Vec<(&'a bbb_protocol::JavaString, V)> {
    let mut visible: Vec<(&'a bbb_protocol::JavaString, V)> = Vec::with_capacity(capacity);
    let mut index: Option<std::collections::HashMap<&'a bbb_protocol::JavaString, usize>> = None;
    for (name, value) in entries {
        if visible.len() >= 32 && index.is_none() {
            let mut map = std::collections::HashMap::with_capacity(visible.len().max(capacity));
            for (pos, &(key, _)) in visible.iter().enumerate() {
                map.insert(key, pos);
            }
            index = Some(map);
        }
        if let Some(map) = index.as_mut() {
            if let Some(&pos) = map.get(name) {
                visible[pos].1 = value;
            } else {
                map.insert(name, visible.len());
                visible.push((name, value));
            }
        } else if let Some((_, previous)) = visible.iter_mut().find(|(key, _)| *key == name) {
            *previous = value;
        } else {
            visible.push((name, value));
        }
    }
    visible
}

impl<'a> From<&'a NbtSummaryValue> for RegistryElementValueRef<'a> {
    fn from(value: &'a NbtSummaryValue) -> Self {
        Self::Nbt(value)
    }
}

impl<'a> RegistryElementValueRef<'a> {
    /// Reads a record field. JSON null is absent, matching
    /// `JsonOps.getMap(...).get(field)`; duplicate NBT fields are last-wins.
    pub fn field(self, name: &str) -> Option<Self> {
        match self {
            Self::Nbt(NbtSummaryValue::Compound(entries)) => entries
                .iter()
                .rev()
                .find(|entry| entry.name == name)
                .map(|entry| Self::Nbt(&entry.value)),
            Self::Json(RegistryJsonValue::Object(entries)) => entries
                .iter()
                .rev()
                .find(|(entry_name, _)| entry_name == name)
                .map(|(_, value)| value)
                .filter(|value| !value.is_null())
                .map(Self::Json),
            _ => None,
        }
    }

    /// Retains explicit JSON null for record codecs with strict optional fields.
    pub(crate) fn record_field(self, name: &str) -> Option<Self> {
        match self {
            Self::Json(RegistryJsonValue::Object(entries)) => entries
                .iter()
                .rev()
                .find(|(entry_name, _)| entry_name == name)
                .map(|(_, value)| Self::Json(value)),
            _ => self.field(name),
        }
    }

    pub fn is_compound(self) -> bool {
        matches!(
            self,
            Self::Nbt(NbtSummaryValue::Compound(_)) | Self::Json(RegistryJsonValue::Object(_))
        )
    }

    pub fn list_len(self) -> Option<usize> {
        match self {
            Self::Nbt(NbtSummaryValue::List(values)) => Some(values.len()),
            Self::Nbt(NbtSummaryValue::ByteArray(values)) => Some(values.len()),
            Self::Nbt(NbtSummaryValue::IntArray(values)) => Some(values.len()),
            Self::Nbt(NbtSummaryValue::LongArray(values)) => Some(values.len()),
            Self::Json(RegistryJsonValue::Array(values)) => Some(values.len()),
            _ => None,
        }
    }

    pub fn list_elements(self) -> Option<impl ExactSizeIterator<Item = Self> + 'a> {
        self.list_len()
            .map(move |len| (0..len).map(move |index| self.list_element(index).unwrap()))
    }

    /// Returns a list element without erasing a JSON null node.
    pub fn list_element(self, index: usize) -> Option<Self> {
        match self {
            Self::Nbt(NbtSummaryValue::List(values)) => values.get(index).map(Self::Nbt),
            Self::Nbt(NbtSummaryValue::ByteArray(values)) => {
                values.get(index).copied().map(Self::NbtByte)
            }
            Self::Nbt(NbtSummaryValue::IntArray(values)) => {
                values.get(index).copied().map(Self::NbtInt)
            }
            Self::Nbt(NbtSummaryValue::LongArray(values)) => {
                values.get(index).copied().map(Self::NbtLong)
            }
            Self::Json(RegistryJsonValue::Array(values)) => values.get(index).map(Self::Json),
            _ => None,
        }
    }

    pub fn map_len(self) -> Option<usize> {
        match self {
            Self::Nbt(NbtSummaryValue::Compound(entries)) => Some(entries.len()),
            Self::Json(RegistryJsonValue::Object(entries)) => Some(entries.len()),
            _ => None,
        }
    }

    /// Returns a raw map entry. Unlike [`Self::field`], a JSON null value is
    /// retained so an unbounded-map value codec can reject it itself.
    pub fn map_entry_java_string_ref(
        self,
        index: usize,
    ) -> Option<(&'a bbb_protocol::JavaString, Self)> {
        match self {
            Self::Nbt(NbtSummaryValue::Compound(entries)) => entries
                .get(index)
                .map(|entry| (&entry.name, Self::Nbt(&entry.value))),
            Self::Json(RegistryJsonValue::Object(entries)) => entries
                .get(index)
                .map(|(name, value)| (name, Self::Json(value))),
            _ => None,
        }
    }

    pub(crate) fn visible_map_entries(
        self,
        len: usize,
    ) -> Vec<(&'a bbb_protocol::JavaString, Self)> {
        collect_visible_map_entries(
            (0..len).map(|index| {
                self.map_entry_java_string_ref(index)
                    .expect("map length and indexed entries stay consistent")
            }),
            len,
        )
    }

    pub fn java_string_ref(self) -> Option<&'a bbb_protocol::JavaString> {
        match self {
            Self::Nbt(NbtSummaryValue::String(value)) => Some(value),
            Self::Json(RegistryJsonValue::String(value)) => Some(value),
            _ => None,
        }
    }

    pub fn utf8_string(self) -> Option<String> {
        self.java_string_ref()?.try_to_string().ok()
    }

    pub fn is_string(self) -> bool {
        matches!(
            self,
            Self::Nbt(NbtSummaryValue::String(_)) | Self::Json(RegistryJsonValue::String(_))
        )
    }

    pub fn java_string(self) -> Option<bbb_protocol::JavaString> {
        self.java_string_ref().cloned()
    }

    pub fn as_bool(self) -> Option<bool> {
        match self {
            Self::Json(RegistryJsonValue::Bool(value)) => Some(*value),
            Self::Nbt(value) => codec_int_value(value).map(|value| value as i8 != 0),
            Self::NbtByte(value) => Some(value != 0),
            Self::NbtInt(value) => Some((value as i8) != 0),
            Self::NbtLong(value) => Some((value as i8) != 0),
            _ => None,
        }
    }

    /// Applies Java `Number.intValue()` narrowing.
    pub fn codec_i32(self) -> Option<i32> {
        match self {
            Self::Nbt(value) => codec_int_value(value),
            Self::NbtByte(value) => Some(i32::from(value)),
            Self::NbtInt(value) => Some(value),
            Self::NbtLong(value) => Some(value as i32),
            Self::Json(RegistryJsonValue::Number(value)) => json_number_i32(value),
            _ => None,
        }
    }

    /// Applies Java `Number.longValue()` narrowing.
    pub fn codec_i64(self) -> Option<i64> {
        match self {
            Self::Nbt(value) => codec_long_value(value),
            Self::NbtByte(value) => Some(i64::from(value)),
            Self::NbtInt(value) => Some(i64::from(value)),
            Self::NbtLong(value) => Some(value),
            Self::Json(RegistryJsonValue::Number(value)) => json_number_i64(value),
            _ => None,
        }
    }

    /// Applies Java `Number.floatValue()` narrowing.
    pub fn codec_f32(self) -> Option<f32> {
        match self {
            Self::Nbt(value) => codec_float_value(value),
            Self::NbtByte(value) => Some(f32::from(value)),
            Self::NbtInt(value) => Some(value as f32),
            Self::NbtLong(value) => Some(value as f32),
            // Gson's LazilyParsedNumber calls Float.parseFloat on the source
            // lexeme directly.  Parsing through f64 first could double-round.
            Self::Json(RegistryJsonValue::Number(value)) => value.parse().ok(),
            _ => None,
        }
    }

    /// Applies Java `Number.doubleValue()` narrowing.
    pub fn codec_f64(self) -> Option<f64> {
        match self {
            Self::Nbt(value) => codec_double_value(value),
            Self::NbtByte(value) => Some(f64::from(value)),
            Self::NbtInt(value) => Some(f64::from(value)),
            Self::NbtLong(value) => Some(value as f64),
            Self::Json(RegistryJsonValue::Number(value)) => value.parse().ok(),
            _ => None,
        }
    }

    pub fn is_null(self) -> bool {
        matches!(self, Self::Json(RegistryJsonValue::Null))
    }
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
    #[error("known-pack registry JSON number has {actual} bytes, maximum is {maximum}")]
    JsonNumberTooLarge { actual: usize, maximum: usize },
}

/// `RegistrySynchronization.PackedRegistryEntry.STREAM_CODEC`
/// (`RegistrySynchronization.java:92-98`) uses `ByteBufCodecs.TAG`
/// (`NbtAccounter.create(2_097_152L)`, max depth 512).
pub fn decode_network_registry_element(
    raw_data: &[u8],
) -> Result<RegistryElementData, RegistryElementDecodeError> {
    let value = decode_nbt_summary_with_limits(
        raw_data,
        NbtSummaryDecodeLimits::NETWORK_QUOTA,
        VANILLA_DEFAULT_NBT_QUOTA,
    )
    .map_err(|error| RegistryElementDecodeError::Network(error.to_string()))?
    .ok_or_else(|| {
        RegistryElementDecodeError::Network("registry element root must not be TAG_End".to_string())
    })?;
    Ok(RegistryElementData::from_network_nbt(value))
}

/// Decodes persistent registry JSON loaded by the server.
///
/// Callers must subsequently apply the generated persistent codec before a
/// synchronized network codec can publish the value.
pub fn decode_direct_registry_json(
    json: &[u8],
) -> Result<RegistryElementData, RegistryElementDecodeError> {
    decode_registry_json(json, RegistryElementSource::ServerDatapackJson)
}

/// Decodes a selected known-pack resource on the client.
///
/// Vanilla still applies the synchronized registry's network element codec to
/// this local JSON source (`NetworkRegistryLoadTask`,
/// `NetworkRegistryLoadTask`). It is therefore distinct from server
/// datapack JSON even though both use JsonOps.
pub fn decode_client_known_pack_registry_json(
    json: &[u8],
) -> Result<RegistryElementData, RegistryElementDecodeError> {
    decode_registry_json(json, RegistryElementSource::ClientKnownPackJson)
}

fn decode_registry_json(
    json: &[u8],
    source: RegistryElementSource,
) -> Result<RegistryElementData, RegistryElementDecodeError> {
    let json_value = parse_registry_json(json)?;
    Ok(RegistryElementData {
        source,
        input: RegistryElementInput::Json(json_value),
    })
}

/// Parses one registry element as Gson does, keeping number lexemes.
pub fn parse_registry_json(json: &[u8]) -> Result<RegistryJsonValue, RegistryElementDecodeError> {
    parse_registry_json_with_limits(
        json,
        MAX_REGISTRY_ELEMENT_JSON_BYTES,
        MAX_REGISTRY_ELEMENT_DEPTH,
        MAX_REGISTRY_ELEMENT_ITEMS,
    )
}

/// The loader's non-exact registries historically use serde's recursion budget
/// and only the resource-byte bound for collection sizes.
pub fn decode_loaded_registry_json(
    json: &[u8],
    maximum_bytes: usize,
) -> Result<RegistryElementData, RegistryElementDecodeError> {
    Ok(RegistryElementData {
        source: RegistryElementSource::ServerDatapackJson,
        input: RegistryElementInput::Json(parse_registry_json_with_limits(
            json,
            maximum_bytes,
            127,
            usize::MAX,
        )?),
    })
}

fn parse_registry_json_with_limits(
    json: &[u8],
    maximum_bytes: usize,
    maximum_depth: usize,
    maximum_items: usize,
) -> Result<RegistryJsonValue, RegistryElementDecodeError> {
    if json.len() > maximum_bytes {
        return Err(RegistryElementDecodeError::JsonTooLarge {
            actual: json.len(),
            maximum: maximum_bytes,
        });
    }
    let text = std::str::from_utf8(json)
        .map_err(|error| RegistryElementDecodeError::Json(error.to_string()))?;
    let mut parser = SinglePassRegistryJsonParser {
        input: text,
        cursor: 0,
        maximum_depth,
        maximum_items,
    };
    let json_value = parser.parse_value(0)?;
    parser.skip_ws();
    if parser.cursor != parser.input.len() {
        return Err(RegistryElementDecodeError::Json(
            "trailing characters after JSON value".to_string(),
        ));
    }
    if json_value.is_null() {
        return Err(RegistryElementDecodeError::NullRoot);
    }
    Ok(json_value)
}

/// Single-pass RFC 8259 JSON parser building [`RegistryJsonValue`] directly
/// while enforcing depth, collection size, and number-token bounds inline and
/// preserving exact number lexemes plus unpaired UTF-16 surrogates.
struct SinglePassRegistryJsonParser<'a> {
    input: &'a str,
    cursor: usize,
    maximum_depth: usize,
    maximum_items: usize,
}

impl<'a> SinglePassRegistryJsonParser<'a> {
    #[inline]
    fn bytes(&self) -> &'a [u8] {
        self.input.as_bytes()
    }

    #[inline]
    fn skip_ws(&mut self) {
        let bytes = self.bytes();
        while self.cursor < bytes.len()
            && matches!(bytes[self.cursor], b' ' | b'\t' | b'\r' | b'\n')
        {
            self.cursor += 1;
        }
    }

    fn parse_value(
        &mut self,
        depth: usize,
    ) -> Result<RegistryJsonValue, RegistryElementDecodeError> {
        self.skip_ws();
        let Some(&byte) = self.bytes().get(self.cursor) else {
            return Err(RegistryElementDecodeError::Json(
                "EOF while parsing a value".to_string(),
            ));
        };
        match byte {
            b'n' => {
                if self.input[self.cursor..].starts_with("null") {
                    self.cursor += 4;
                    Ok(RegistryJsonValue::Null)
                } else {
                    Err(RegistryElementDecodeError::Json(
                        "expected `null`".to_string(),
                    ))
                }
            }
            b't' => {
                if self.input[self.cursor..].starts_with("true") {
                    self.cursor += 4;
                    Ok(RegistryJsonValue::Bool(true))
                } else {
                    Err(RegistryElementDecodeError::Json(
                        "expected `true`".to_string(),
                    ))
                }
            }
            b'f' => {
                if self.input[self.cursor..].starts_with("false") {
                    self.cursor += 5;
                    Ok(RegistryJsonValue::Bool(false))
                } else {
                    Err(RegistryElementDecodeError::Json(
                        "expected `false`".to_string(),
                    ))
                }
            }
            b'"' => self.parse_string().map(RegistryJsonValue::String),
            b'[' => self.parse_array(depth),
            b'{' => self.parse_object(depth),
            b'-' | b'0'..=b'9' => self.parse_number(),
            _ => Err(RegistryElementDecodeError::Json(
                "unexpected character while parsing JSON value".to_string(),
            )),
        }
    }

    fn parse_array(
        &mut self,
        depth: usize,
    ) -> Result<RegistryJsonValue, RegistryElementDecodeError> {
        let next_depth = depth + 1;
        if next_depth > self.maximum_depth {
            return Err(RegistryElementDecodeError::JsonDepth {
                maximum: self.maximum_depth,
            });
        }
        self.cursor += 1; // '['
        self.skip_ws();
        let mut items = Vec::new();
        if self.bytes().get(self.cursor) == Some(&b']') {
            self.cursor += 1;
            return Ok(RegistryJsonValue::Array(items));
        }
        loop {
            let value = self.parse_value(next_depth)?;
            items.push(value);
            if items.len() > self.maximum_items {
                return Err(RegistryElementDecodeError::JsonCollectionTooLarge {
                    actual: items.len(),
                    maximum: self.maximum_items,
                });
            }
            self.skip_ws();
            match self.bytes().get(self.cursor) {
                Some(&b',') => self.cursor += 1,
                Some(&b']') => {
                    self.cursor += 1;
                    return Ok(RegistryJsonValue::Array(items));
                }
                _ => {
                    return Err(RegistryElementDecodeError::Json(
                        "expected `,` or `]` in JSON array".to_string(),
                    ));
                }
            }
        }
    }

    /// Retains Gson `JsonObject` insertion order: first occurrence sets the
    /// slot position and last occurrence sets the value.
    fn parse_object(
        &mut self,
        depth: usize,
    ) -> Result<RegistryJsonValue, RegistryElementDecodeError> {
        let next_depth = depth + 1;
        if next_depth > self.maximum_depth {
            return Err(RegistryElementDecodeError::JsonDepth {
                maximum: self.maximum_depth,
            });
        }
        self.cursor += 1; // '{'
        self.skip_ws();
        let mut entries: Vec<(bbb_protocol::JavaString, RegistryJsonValue)> = Vec::new();
        let mut positions: Option<std::collections::HashMap<bbb_protocol::JavaString, usize>> =
            None;
        if self.bytes().get(self.cursor) == Some(&b'}') {
            self.cursor += 1;
            return Ok(RegistryJsonValue::Object(entries));
        }
        loop {
            self.skip_ws();
            if self.bytes().get(self.cursor) != Some(&b'"') {
                return Err(RegistryElementDecodeError::Json(
                    "expected string key in JSON object".to_string(),
                ));
            }
            let name = self.parse_string()?;
            self.skip_ws();
            if self.bytes().get(self.cursor) != Some(&b':') {
                return Err(RegistryElementDecodeError::Json(
                    "expected `:` after JSON object key".to_string(),
                ));
            }
            self.cursor += 1;
            let value = self.parse_value(next_depth)?;
            if entries.len() >= 32 && positions.is_none() {
                positions = Some(
                    entries
                        .iter()
                        .enumerate()
                        .map(|(index, (key, _))| (key.clone(), index))
                        .collect(),
                );
            }
            let prior = match &positions {
                Some(positions) => positions.get(&name).copied(),
                None => entries
                    .iter()
                    .position(|(prior_name, _)| prior_name == &name),
            };
            if let Some(index) = prior {
                entries[index].1 = value;
            } else {
                if let Some(positions) = &mut positions {
                    positions.insert(name.clone(), entries.len());
                }
                entries.push((name, value));
                if entries.len() > self.maximum_items {
                    return Err(RegistryElementDecodeError::JsonCollectionTooLarge {
                        actual: entries.len(),
                        maximum: self.maximum_items,
                    });
                }
            }
            self.skip_ws();
            match self.bytes().get(self.cursor) {
                Some(&b',') => self.cursor += 1,
                Some(&b'}') => {
                    self.cursor += 1;
                    return Ok(RegistryJsonValue::Object(entries));
                }
                _ => {
                    return Err(RegistryElementDecodeError::Json(
                        "expected `,` or `}` in JSON object".to_string(),
                    ));
                }
            }
        }
    }

    fn parse_number(&mut self) -> Result<RegistryJsonValue, RegistryElementDecodeError> {
        let bytes = self.bytes();
        let start = self.cursor;
        let mut idx = start;
        if bytes.get(idx) == Some(&b'-') {
            idx += 1;
        }
        match bytes.get(idx) {
            Some(b'0') => idx += 1,
            Some(b'1'..=b'9') => {
                idx += 1;
                while bytes.get(idx).is_some_and(u8::is_ascii_digit) {
                    idx += 1;
                }
            }
            _ => {
                return Err(RegistryElementDecodeError::Json(
                    "invalid JSON number".to_string(),
                ));
            }
        }
        if bytes.get(idx) == Some(&b'.') {
            idx += 1;
            let frac_start = idx;
            while bytes.get(idx).is_some_and(u8::is_ascii_digit) {
                idx += 1;
            }
            if idx == frac_start {
                return Err(RegistryElementDecodeError::Json(
                    "invalid JSON number fraction".to_string(),
                ));
            }
        }
        if matches!(bytes.get(idx), Some(b'e' | b'E')) {
            idx += 1;
            if matches!(bytes.get(idx), Some(b'+' | b'-')) {
                idx += 1;
            }
            let exp_start = idx;
            while bytes.get(idx).is_some_and(u8::is_ascii_digit) {
                idx += 1;
            }
            if idx == exp_start {
                return Err(RegistryElementDecodeError::Json(
                    "invalid JSON number exponent".to_string(),
                ));
            }
        }
        let len = idx - start;
        if len > MAX_REGISTRY_ELEMENT_JSON_NUMBER_BYTES {
            return Err(RegistryElementDecodeError::JsonNumberTooLarge {
                actual: len,
                maximum: MAX_REGISTRY_ELEMENT_JSON_NUMBER_BYTES,
            });
        }
        self.cursor = idx;
        Ok(RegistryJsonValue::Number(
            self.input[start..idx].to_string(),
        ))
    }

    fn parse_string(&mut self) -> Result<bbb_protocol::JavaString, RegistryElementDecodeError> {
        let bytes = self.bytes();
        let content_start = self.cursor + 1;
        let mut idx = content_start;
        // Fast path: scan unescaped string slice directly into JavaString.
        while idx < bytes.len() {
            let byte = bytes[idx];
            if byte == b'"' {
                self.cursor = idx + 1;
                return Ok(bbb_protocol::JavaString::from(
                    &self.input[content_start..idx],
                ));
            }
            if byte == b'\\' {
                break;
            }
            if byte < 0x20 {
                return Err(RegistryElementDecodeError::Json(
                    "control character in JSON string".to_string(),
                ));
            }
            idx += 1;
        }
        if idx >= bytes.len() {
            return Err(RegistryElementDecodeError::Json(
                "EOF while parsing JSON string".to_string(),
            ));
        }

        let mut units = Vec::with_capacity(idx - content_start + 16);
        units.extend(self.input[content_start..idx].encode_utf16());
        while idx < bytes.len() {
            let byte = bytes[idx];
            if byte == b'"' {
                self.cursor = idx + 1;
                return Ok(bbb_protocol::JavaString::from(units));
            }
            if byte == b'\\' {
                idx += 1;
                let Some(&escaped) = bytes.get(idx) else {
                    return Err(RegistryElementDecodeError::Json(
                        "EOF while parsing JSON escape".to_string(),
                    ));
                };
                match escaped {
                    b'"' => {
                        units.push(u16::from(b'"'));
                        idx += 1;
                    }
                    b'\\' => {
                        units.push(u16::from(b'\\'));
                        idx += 1;
                    }
                    b'/' => {
                        units.push(u16::from(b'/'));
                        idx += 1;
                    }
                    b'b' => {
                        units.push(0x0008);
                        idx += 1;
                    }
                    b'f' => {
                        units.push(0x000c);
                        idx += 1;
                    }
                    b'n' => {
                        units.push(u16::from(b'\n'));
                        idx += 1;
                    }
                    b'r' => {
                        units.push(u16::from(b'\r'));
                        idx += 1;
                    }
                    b't' => {
                        units.push(u16::from(b'\t'));
                        idx += 1;
                    }
                    b'u' => {
                        let hex = bytes.get(idx + 1..idx + 5).ok_or_else(|| {
                            RegistryElementDecodeError::Json(
                                "EOF while parsing unicode escape".to_string(),
                            )
                        })?;
                        let mut code_unit = 0u16;
                        for &h in hex {
                            let nibble = match h {
                                b'0'..=b'9' => h - b'0',
                                b'a'..=b'f' => h - b'a' + 10,
                                b'A'..=b'F' => h - b'A' + 10,
                                _ => {
                                    return Err(RegistryElementDecodeError::Json(
                                        "invalid hex digit in unicode escape".to_string(),
                                    ));
                                }
                            };
                            code_unit = (code_unit << 4) | u16::from(nibble);
                        }
                        units.push(code_unit);
                        idx += 5;
                    }
                    _ => {
                        return Err(RegistryElementDecodeError::Json(
                            "invalid escape sequence in JSON string".to_string(),
                        ));
                    }
                }
                continue;
            }
            if byte < 0x20 {
                return Err(RegistryElementDecodeError::Json(
                    "control character in JSON string".to_string(),
                ));
            }
            let chunk_start = idx;
            while idx < bytes.len()
                && bytes[idx] != b'"'
                && bytes[idx] != b'\\'
                && bytes[idx] >= 0x20
            {
                idx += 1;
            }
            units.extend(self.input[chunk_start..idx].encode_utf16());
        }
        Err(RegistryElementDecodeError::Json(
            "EOF while parsing JSON string".to_string(),
        ))
    }
}

fn json_to_element_value(
    value: &RegistryJsonValue,
    depth: usize,
) -> Result<NbtSummaryValue, RegistryElementDecodeError> {
    match value {
        RegistryJsonValue::Null => Err(RegistryElementDecodeError::Json(
            "null has no NBT value representation".to_string(),
        )),
        RegistryJsonValue::Bool(value) => Ok(NbtSummaryValue::Long(i64::from(*value))),
        RegistryJsonValue::Number(value) => json_number_to_element_value(value),
        RegistryJsonValue::String(value) => Ok(NbtSummaryValue::String(value.clone())),
        RegistryJsonValue::Array(values) => {
            let nested_depth = enter_json_container(depth)?;
            check_collection_len(values.len())?;
            let mut converted = Vec::with_capacity(values.len());
            for value in values {
                converted.push(json_to_element_value(value, nested_depth)?);
            }
            Ok(NbtSummaryValue::List(converted))
        }
        RegistryJsonValue::Object(entries) => {
            let nested_depth = enter_json_container(depth)?;
            check_collection_len(entries.len())?;
            let mut converted = Vec::with_capacity(entries.len());
            for (name, value) in entries {
                if value.is_null() {
                    continue;
                }
                converted.push(NbtSummaryEntry {
                    name: name.clone(),
                    value: json_to_element_value(value, nested_depth)?,
                });
            }
            Ok(NbtSummaryValue::Compound(converted))
        }
    }
}

fn json_number_to_element_value(
    value: &str,
) -> Result<NbtSummaryValue, RegistryElementDecodeError> {
    if let Ok(value) = value.parse::<i64>() {
        return Ok(NbtSummaryValue::Long(value));
    }
    if let Ok(value) = value.parse::<u64>() {
        return Ok(NbtSummaryValue::Long(value as i64));
    }
    let value = value.parse::<f64>().map_err(|_| {
        RegistryElementDecodeError::Json("number has no f64 representation".to_string())
    })?;
    Ok(NbtSummaryValue::Double(canonical_double_tag_bits(value)))
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

fn canonical_element_value(value: &NbtSummaryValue) -> NbtSummaryValue {
    match value {
        NbtSummaryValue::Byte(number) => NbtSummaryValue::Long(i64::from(*number)),
        NbtSummaryValue::Short(number) => NbtSummaryValue::Long(i64::from(*number)),
        NbtSummaryValue::Int(number) => NbtSummaryValue::Long(i64::from(*number)),
        NbtSummaryValue::Float(bits) => {
            NbtSummaryValue::Double(f64::from(f32::from_bits(*bits)).to_bits())
        }
        NbtSummaryValue::ByteArray(numbers) => NbtSummaryValue::List(
            numbers
                .iter()
                .map(|number| NbtSummaryValue::Long(i64::from(*number)))
                .collect(),
        ),
        NbtSummaryValue::IntArray(numbers) => NbtSummaryValue::List(
            numbers
                .iter()
                .map(|number| NbtSummaryValue::Long(i64::from(*number)))
                .collect(),
        ),
        NbtSummaryValue::LongArray(numbers) => {
            NbtSummaryValue::List(numbers.iter().copied().map(NbtSummaryValue::Long).collect())
        }
        NbtSummaryValue::List(values) => {
            NbtSummaryValue::List(values.iter().map(canonical_element_value).collect())
        }
        NbtSummaryValue::Compound(entries) => NbtSummaryValue::Compound(
            entries
                .iter()
                .map(|field| entry(field.name.clone(), canonical_element_value(&field.value)))
                .collect(),
        ),
        NbtSummaryValue::Long(_) | NbtSummaryValue::Double(_) | NbtSummaryValue::String(_) => {
            value.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_projection_preserves_numeric_values_and_raw_keys() {
        let json = decode_direct_registry_json(
            br#"{"omitted":null,"\ud800":[true,-7,42,4294967296,18446744073709551615,1.25],"\ud801":[[-1,2],[-3,4],[-5,6]]}"#,
        )
        .unwrap();
        let nbt = RegistryElementData::from_network_nbt(NbtSummaryValue::Compound(vec![
            entry(
                bbb_protocol::JavaString::from(vec![0xd800]),
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::Byte(1),
                    NbtSummaryValue::Short(-7),
                    NbtSummaryValue::Int(42),
                    NbtSummaryValue::Long(4294967296),
                    NbtSummaryValue::Long(-1),
                    NbtSummaryValue::Float(1.25f32.to_bits()),
                ]),
            ),
            entry(
                bbb_protocol::JavaString::from(vec![0xd801]),
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::ByteArray(vec![-1, 2]),
                    NbtSummaryValue::IntArray(vec![-3, 4]),
                    NbtSummaryValue::LongArray(vec![-5, 6]),
                ]),
            ),
        ]));
        let expected = NbtSummaryValue::Compound(vec![
            entry(
                bbb_protocol::JavaString::from(vec![0xd800]),
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::Long(1),
                    NbtSummaryValue::Long(-7),
                    NbtSummaryValue::Long(42),
                    NbtSummaryValue::Long(4294967296),
                    NbtSummaryValue::Long(-1),
                    NbtSummaryValue::Double(1.25f64.to_bits()),
                ]),
            ),
            entry(
                bbb_protocol::JavaString::from(vec![0xd801]),
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::List(vec![
                        NbtSummaryValue::Long(-1),
                        NbtSummaryValue::Long(2),
                    ]),
                    NbtSummaryValue::List(vec![
                        NbtSummaryValue::Long(-3),
                        NbtSummaryValue::Long(4),
                    ]),
                    NbtSummaryValue::List(vec![
                        NbtSummaryValue::Long(-5),
                        NbtSummaryValue::Long(6),
                    ]),
                ]),
            ),
        ]);
        assert_eq!(json.source_order_nbt_projection().unwrap(), expected);
        assert_eq!(nbt.source_order_nbt_projection().unwrap(), expected);

        let nested_null_array =
            decode_direct_registry_json(br#"{"valid":1,"nested":[[1,null]]}"#).unwrap();
        assert!(nested_null_array.source_order_nbt_projection().is_err());
    }

    #[test]
    fn source_json_object_and_map_entries_retain_insertion_order() {
        let data = decode_direct_registry_json(br#"{"z":1,"a":2}"#).unwrap();
        let RegistryElementValueRef::Json(RegistryJsonValue::Object(object)) = data.root() else {
            panic!("expected JSON source");
        };
        assert_eq!(
            object
                .iter()
                .map(|(name, _)| name.units())
                .collect::<Vec<_>>(),
            [[b'z' as u16].as_slice(), [b'a' as u16].as_slice()]
        );

        let root = data.root();
        assert_eq!(
            root.map_entry_java_string_ref(0).map(|(name, _)| name),
            Some(&bbb_protocol::JavaString::from("z"))
        );
        assert_eq!(
            root.map_entry_java_string_ref(1).map(|(name, _)| name),
            Some(&bbb_protocol::JavaString::from("a"))
        );

        let raw = decode_direct_registry_json(
            br#"{"\ud800":"\ud801","z":[1E+02,-0,1e400,null,true],"a":"\\uD800"}"#,
        )
        .unwrap();
        let RegistryElementValueRef::Json(value) = raw.root() else {
            panic!("expected JSON source");
        };
        let encoded = serde_json::to_vec(value).unwrap();
        assert!(std::str::from_utf8(&encoded)
            .unwrap()
            .contains("1E+02,-0,1e400"));
        assert_eq!(decode_direct_registry_json(&encoded).unwrap(), raw);
    }

    #[test]
    fn decode_network_registry_element_uses_vanilla_2mib_quota_and_512_depth() {
        // Depth 80 exceeds the old 64-depth cap but fits Vanilla's 512 limit.
        let mut deep = Vec::new();
        deep.push(10);
        for _ in 0..80 {
            deep.extend_from_slice(&[10, 0, 1, b'a']);
        }
        deep.extend(std::iter::repeat_n(0u8, 81));
        assert!(decode_network_registry_element(&deep).is_ok());

        // >4096 compound entries within 2 MiB accounted bytes decode cleanly.
        let mut wide = Vec::new();
        wide.push(10);
        for i in 0..5000u32 {
            let key = format!("k{i}");
            wide.push(1);
            wide.extend_from_slice(&(key.len() as u16).to_be_bytes());
            wide.extend_from_slice(key.as_bytes());
            wide.push(1);
        }
        wide.push(0);
        assert!(decode_network_registry_element(&wide).is_ok());

        // Exceeding 2 MiB accounted bytes is rejected.
        let mut over = Vec::new();
        over.push(10);
        over.extend_from_slice(&[7, 0, 1, b'b']);
        over.extend_from_slice(&(2_100_000i32).to_be_bytes());
        over.resize(over.len() + 2_100_000, 0);
        over.push(0);
        assert!(decode_network_registry_element(&over).is_err());
    }
}
