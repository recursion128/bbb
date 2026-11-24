use serde::{Deserialize, Serialize};
use std::borrow::{Borrow, Cow};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

use crate::NbtSummaryValue;

const INLINE_UNITS: usize = 11;
const INLINE_VALID_UTF8_BIT: u8 = 0x80;
const INLINE_LEN_MASK: u8 = 0x7f;

#[derive(Clone)]
enum JavaStringRepr {
    Inline {
        meta: u8,
        units: [u16; INLINE_UNITS],
    },
    Heap {
        valid_utf8: bool,
        units: Box<[u16]>,
    },
}

/// A Java string's original UTF-16 code units, stored inline up to 11 units (22 bytes).
#[derive(Clone)]
pub struct JavaString(JavaStringRepr);

#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<JavaString>() == 24);

impl Default for JavaString {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}

impl PartialEq for JavaString {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.units() == other.units()
    }
}

impl Eq for JavaString {}

impl PartialOrd for JavaString {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for JavaString {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.units().cmp(other.units())
    }
}

impl Hash for JavaString {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.units().hash(state);
    }
}

#[inline]
fn units_are_valid_utf16(units: &[u16]) -> bool {
    if units.iter().all(|&unit| (unit & 0xf800) != 0xd800) {
        return true;
    }
    char::decode_utf16(units.iter().copied()).all(|decoded| decoded.is_ok())
}

impl Serialize for JavaString {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.is_valid_utf8() {
            return Self::with_utf8_units(self.units(), |value| serializer.serialize_str(value))
                .expect("validated UTF-16 converts to UTF-8");
        }
        use std::fmt::Write;
        let units = self.units();
        let mut raw = String::with_capacity(units.len() * 6 + 2);
        raw.push('"');
        for unit in units {
            write!(raw, "\\u{unit:04x}").expect("writing to a String");
        }
        raw.push('"');
        serde_json::value::RawValue::from_string(raw)
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for JavaString {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct StringVisitor;
        impl serde::de::Visitor<'_> for StringVisitor {
            type Value = JavaString;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a JSON string")
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(value.into())
            }

            fn visit_bytes<E: serde::de::Error>(self, mut bytes: &[u8]) -> Result<Self::Value, E> {
                if let Ok(value) = std::str::from_utf8(bytes) {
                    return Ok(value.into());
                }
                let mut units = Vec::with_capacity(bytes.len());
                loop {
                    match std::str::from_utf8(bytes) {
                        Ok(value) => {
                            units.extend(value.encode_utf16());
                            return Ok(JavaString::from(units));
                        }
                        Err(error) => {
                            let (valid, remaining) = bytes.split_at(error.valid_up_to());
                            units.extend(std::str::from_utf8(valid).unwrap().encode_utf16());
                            let Some([0xed, second @ 0xa0..=0xbf, third @ 0x80..=0xbf]) =
                                remaining.get(..3)
                            else {
                                return Err(E::custom("invalid WTF-8 JSON string"));
                            };
                            units.push(
                                0xd000
                                    | (u16::from(*second & 0x3f) << 6)
                                    | u16::from(*third & 0x3f),
                            );
                            bytes = &remaining[3..];
                        }
                    }
                }
            }
        }
        // serde_json preserves escaped surrogates as WTF-8 for bytes visitors.
        // Without visit_seq, JSON arrays cannot masquerade as strings.
        deserializer.deserialize_bytes(StringVisitor)
    }
}

impl std::fmt::Debug for JavaString {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:?}", self.to_display_string())
    }
}

impl std::fmt::Display for JavaString {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write;
        for value in char::decode_utf16(self.units().iter().copied()) {
            formatter.write_char(value.unwrap_or(char::REPLACEMENT_CHARACTER))?;
        }
        Ok(())
    }
}

impl JavaString {
    #[inline]
    const fn empty() -> Self {
        Self(JavaStringRepr::Inline {
            meta: INLINE_VALID_UTF8_BIT,
            units: [0; INLINE_UNITS],
        })
    }

    #[inline]
    pub(crate) fn is_valid_utf8(&self) -> bool {
        match &self.0 {
            JavaStringRepr::Inline { meta, .. } => (*meta & INLINE_VALID_UTF8_BIT) != 0,
            JavaStringRepr::Heap { valid_utf8, .. } => *valid_utf8,
        }
    }

    pub(crate) fn from_ascii_bytes(bytes: &[u8]) -> Self {
        if bytes.len() <= INLINE_UNITS {
            let mut units = [0_u16; INLINE_UNITS];
            for (dst, &src) in units.iter_mut().zip(bytes) {
                *dst = u16::from(src);
            }
            return Self(JavaStringRepr::Inline {
                meta: (bytes.len() as u8) | INLINE_VALID_UTF8_BIT,
                units,
            });
        }
        let units: Box<[u16]> = bytes.iter().map(|&b| u16::from(b)).collect();
        Self(JavaStringRepr::Heap {
            valid_utf8: true,
            units,
        })
    }

    pub(crate) fn from_str_with_utf16_len(value: &str, utf16_len: usize) -> Self {
        if value.is_ascii() {
            return Self::from_ascii_bytes(value.as_bytes());
        }
        if utf16_len <= INLINE_UNITS {
            let mut units = [0_u16; INLINE_UNITS];
            let mut len = 0;
            for unit in value.encode_utf16() {
                units[len] = unit;
                len += 1;
            }
            debug_assert_eq!(len, utf16_len);
            return Self(JavaStringRepr::Inline {
                meta: (len as u8) | INLINE_VALID_UTF8_BIT,
                units,
            });
        }
        let mut units = Vec::with_capacity(utf16_len);
        units.extend(value.encode_utf16());
        debug_assert_eq!(units.len(), utf16_len);
        Self(JavaStringRepr::Heap {
            valid_utf8: true,
            units: units.into_boxed_slice(),
        })
    }

    pub(crate) fn from_valid_utf16_vec(units: Vec<u16>) -> Self {
        Self::from_vec_with_validity(units, true)
    }

    fn from_vec_with_validity(value: Vec<u16>, valid_utf8: bool) -> Self {
        if value.len() <= INLINE_UNITS {
            let mut units = [0_u16; INLINE_UNITS];
            units[..value.len()].copy_from_slice(&value);
            let flag = if valid_utf8 { INLINE_VALID_UTF8_BIT } else { 0 };
            return Self(JavaStringRepr::Inline {
                meta: (value.len() as u8) | flag,
                units,
            });
        }
        Self(JavaStringRepr::Heap {
            valid_utf8,
            units: value.into_boxed_slice(),
        })
    }

    fn from_slice_with_validity(value: &[u16], valid_utf8: bool) -> Self {
        if value.len() <= INLINE_UNITS {
            let mut units = [0_u16; INLINE_UNITS];
            units[..value.len()].copy_from_slice(value);
            let flag = if valid_utf8 { INLINE_VALID_UTF8_BIT } else { 0 };
            return Self(JavaStringRepr::Inline {
                meta: (value.len() as u8) | flag,
                units,
            });
        }
        Self(JavaStringRepr::Heap {
            valid_utf8,
            units: Box::from(value),
        })
    }

    #[inline]
    pub fn units(&self) -> &[u16] {
        match &self.0 {
            JavaStringRepr::Inline { meta, units } => {
                &units[..usize::from(*meta & INLINE_LEN_MASK)]
            }
            JavaStringRepr::Heap { units, .. } => units,
        }
    }

    pub fn capacity(&self) -> usize {
        match &self.0 {
            JavaStringRepr::Inline { .. } => INLINE_UNITS,
            JavaStringRepr::Heap { units, .. } => units.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.units().is_empty()
    }

    /// `StringUtil.isBlank` uses `Character.isWhitespace || isSpaceChar`.
    pub fn is_minecraft_blank(&self) -> bool {
        Self::minecraft_units_are_blank(self.units())
    }

    pub fn minecraft_units_are_blank(units: &[u16]) -> bool {
        units.iter().all(|unit| {
            java_character_is_whitespace(*unit) || matches!(unit, 0x00a0 | 0x2007 | 0x202f)
        })
    }

    pub fn push_str(&mut self, suffix: &str) {
        if suffix.is_empty() {
            return;
        }
        if let JavaStringRepr::Inline { meta, units } = &mut self.0 {
            let len = usize::from(*meta & INLINE_LEN_MASK);
            if suffix.is_ascii() && len + suffix.len() <= INLINE_UNITS {
                for (dst, src) in units[len..len + suffix.len()]
                    .iter_mut()
                    .zip(suffix.bytes())
                {
                    *dst = u16::from(src);
                }
                *meta = ((len + suffix.len()) as u8) | (*meta & INLINE_VALID_UTF8_BIT);
                return;
            }
        }
        let valid_utf8 = self.is_valid_utf8();
        let existing = self.units();
        let mut combined = Vec::with_capacity(existing.len() + suffix.len());
        combined.extend_from_slice(existing);
        combined.extend(suffix.encode_utf16());
        *self = Self::from_vec_with_validity(combined, valid_utf8);
    }

    pub fn strip_prefix(&self, prefix: char) -> Option<Self> {
        let mut buffer = [0; 2];
        let prefix = prefix.encode_utf16(&mut buffer);
        let valid_utf8 = self.is_valid_utf8();
        self.units()
            .strip_prefix(&*prefix)
            .map(|units| Self::from_slice_with_validity(units, valid_utf8))
    }

    pub fn from_nbt(value: &NbtSummaryValue) -> Option<Self> {
        match value {
            NbtSummaryValue::String(value) => Some(value.clone()),
            _ => None,
        }
    }

    pub fn to_jdk_string(&self) -> String {
        String::from_utf16_lossy(self.units())
    }

    pub fn try_to_string(&self) -> Result<String, std::string::FromUtf16Error> {
        String::from_utf16(self.units())
    }

    pub fn with_utf8<T>(&self, apply: impl FnOnce(&str) -> T) -> Option<T> {
        if !self.is_valid_utf8() {
            return None;
        }
        Self::with_utf8_units(self.units(), apply)
    }

    pub fn with_utf8_units<T>(units: &[u16], apply: impl FnOnce(&str) -> T) -> Option<T> {
        let mut ascii = [0_u8; 128];
        if units.len() <= ascii.len() && units.iter().all(|unit| *unit <= 0x7f) {
            for (byte, unit) in ascii.iter_mut().zip(units) {
                *byte = *unit as u8;
            }
            return Some(apply(
                std::str::from_utf8(&ascii[..units.len()]).expect("ASCII is UTF-8"),
            ));
        }
        String::from_utf16(units).ok().map(|value| apply(&value))
    }

    pub fn with_utf16<T>(value: &str, apply: impl FnOnce(&[u16]) -> T) -> T {
        let mut units = [0_u16; 64];
        let mut encoded = value.encode_utf16();
        let mut len = 0;
        while let Some(unit) = encoded.next() {
            if len == units.len() {
                let mut heap = Vec::with_capacity(value.len());
                heap.extend_from_slice(&units);
                heap.push(unit);
                heap.extend(encoded);
                return apply(&heap);
            }
            units[len] = unit;
            len += 1;
        }
        apply(&units[..len])
    }

    pub fn to_jdk_utf8(&self) -> Vec<u8> {
        let units = self.units();
        let mut bytes = Vec::with_capacity(units.len());
        encode_jdk_utf8(units, |encoded| bytes.extend_from_slice(encoded));
        bytes
    }

    pub fn to_display_string(&self) -> String {
        self.to_jdk_string()
    }

    pub fn write_display_to(&self, output: &mut String) {
        output.extend(
            char::decode_utf16(self.units().iter().copied())
                .map(|value| value.unwrap_or(char::REPLACEMENT_CHARACTER)),
        );
    }

    pub fn to_nbt(&self) -> NbtSummaryValue {
        NbtSummaryValue::String(self.clone())
    }

    /// `String.toLowerCase(Locale.ROOT)`.
    pub fn to_lowercase_root(&self) -> Self {
        let source = self.units();
        if source.iter().all(|unit| *unit <= 0x7f) {
            if source.len() <= INLINE_UNITS {
                let mut inline = [0_u16; INLINE_UNITS];
                for (dst, &unit) in inline.iter_mut().zip(source) {
                    *dst = if (0x41..=0x5a).contains(&unit) {
                        unit + 0x20
                    } else {
                        unit
                    };
                }
                return Self(JavaStringRepr::Inline {
                    meta: (source.len() as u8) | INLINE_VALID_UTF8_BIT,
                    units: inline,
                });
            }
            let mut units = source.to_vec();
            for unit in &mut units {
                if (0x41..=0x5a).contains(unit) {
                    *unit += 0x20;
                }
            }
            return Self::from_valid_utf16_vec(units);
        }
        if self.is_valid_utf8() {
            if let Ok(value) = self.try_to_string() {
                return value.to_lowercase().into();
            }
        }
        let mut units = Vec::with_capacity(source.len());
        let mut run = String::new();
        for value in char::decode_utf16(source.iter().copied()) {
            match value {
                Ok(value) => run.push(value),
                Err(error) => {
                    units.extend(run.to_lowercase().encode_utf16());
                    run.clear();
                    units.push(error.unpaired_surrogate());
                }
            }
        }
        units.extend(run.to_lowercase().encode_utf16());
        Self::from_vec_with_validity(units, false)
    }
}

impl From<Vec<u16>> for JavaString {
    fn from(value: Vec<u16>) -> Self {
        let valid_utf8 = units_are_valid_utf16(&value);
        Self::from_vec_with_validity(value, valid_utf8)
    }
}

impl From<&[u16]> for JavaString {
    fn from(value: &[u16]) -> Self {
        let valid_utf8 = units_are_valid_utf16(value);
        Self::from_slice_with_validity(value, valid_utf8)
    }
}

impl From<String> for JavaString {
    fn from(value: String) -> Self {
        value.as_str().into()
    }
}

impl From<&str> for JavaString {
    fn from(value: &str) -> Self {
        if value.is_ascii() {
            return Self::from_ascii_bytes(value.as_bytes());
        }
        if value.len() <= INLINE_UNITS {
            let mut units = [0_u16; INLINE_UNITS];
            let mut len = 0;
            for unit in value.encode_utf16() {
                units[len] = unit;
                len += 1;
            }
            return Self(JavaStringRepr::Inline {
                meta: (len as u8) | INLINE_VALID_UTF8_BIT,
                units,
            });
        }
        let utf16_len = crate::codec::utf8_utf16_len(value);
        Self::from_str_with_utf16_len(value, utf16_len)
    }
}

impl From<&String> for JavaString {
    fn from(value: &String) -> Self {
        value.as_str().into()
    }
}

impl From<&JavaString> for JavaString {
    fn from(value: &JavaString) -> Self {
        value.clone()
    }
}

impl<'a> TryFrom<&'a JavaString> for Cow<'a, str> {
    type Error = std::string::FromUtf16Error;

    fn try_from(value: &'a JavaString) -> Result<Self, Self::Error> {
        value.try_to_string().map(Cow::Owned)
    }
}

impl From<JavaString> for Vec<u16> {
    fn from(value: JavaString) -> Self {
        match value.0 {
            JavaStringRepr::Inline { meta, units } => {
                units[..usize::from(meta & INLINE_LEN_MASK)].to_vec()
            }
            JavaStringRepr::Heap { units, .. } => units.into_vec(),
        }
    }
}

impl Borrow<[u16]> for JavaString {
    fn borrow(&self) -> &[u16] {
        self.units()
    }
}

impl PartialEq<str> for JavaString {
    fn eq(&self, other: &str) -> bool {
        if !self.is_valid_utf8() {
            return false;
        }
        let units = self.units();
        if other.is_ascii() {
            return units.len() == other.len()
                && units
                    .iter()
                    .zip(other.bytes())
                    .all(|(&unit, byte)| unit == u16::from(byte));
        }
        units.iter().copied().eq(other.encode_utf16())
    }
}

impl PartialEq<&str> for JavaString {
    fn eq(&self, other: &&str) -> bool {
        self == *other
    }
}

impl PartialEq<String> for JavaString {
    fn eq(&self, other: &String) -> bool {
        self == other.as_str()
    }
}

impl JavaString {
    pub fn from_json_string(raw: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(raw)
    }

    pub fn deserialize_json_string<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        Self::deserialize(deserializer)
    }
}

pub fn serialize_java_string_map<'a, S, V, I>(entries: I, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
    V: Serialize + 'a,
    I: Iterator<Item = (&'a JavaString, &'a V)> + Clone,
{
    if entries.clone().all(|(key, _)| key.is_valid_utf8()) {
        return serializer.collect_map(entries);
    }
    // serde_json's map-key serializer cannot emit unpaired UTF-16 units.
    let mut raw = vec![b'{'];
    for (index, (key, value)) in entries.enumerate() {
        if index != 0 {
            raw.push(b',');
        }
        serde_json::to_writer(&mut raw, key).map_err(serde::ser::Error::custom)?;
        raw.push(b':');
        serde_json::to_writer(&mut raw, value).map_err(serde::ser::Error::custom)?;
    }
    raw.push(b'}');
    serde_json::value::RawValue::from_string(
        String::from_utf8(raw).expect("JSON serialization writes UTF-8"),
    )
    .map_err(serde::ser::Error::custom)?
    .serialize(serializer)
}

/// Projects unpaired UTF-16 surrogates to replacement characters at a JSON display boundary.
pub fn to_display_json_value<T: Serialize + ?Sized>(
    value: &T,
) -> Result<serde_json::Value, serde_json::Error> {
    let error = match serde_json::to_value(value) {
        Ok(value) => return Ok(value),
        Err(error) if error.is_syntax() => error,
        Err(error) => return Err(error),
    };
    let raw = serde_json::value::to_raw_value(value)?;
    let mut replaced = false;
    let display = display_raw_json_value(&raw, &mut replaced)?;
    if replaced {
        Ok(display)
    } else {
        Err(error)
    }
}

fn display_raw_json_value(
    raw: &serde_json::value::RawValue,
    replaced: &mut bool,
) -> Result<serde_json::Value, serde_json::Error> {
    let mut parser = SinglePassDisplayJsonParser {
        input: raw.get(),
        cursor: 0,
        replaced,
    };
    let value = parser.parse_value()?;
    parser.skip_ws();
    if parser.cursor != parser.input.len() {
        return serde_json::from_str(&parser.input[parser.cursor..]);
    }
    Ok(value)
}

struct SinglePassDisplayJsonParser<'a> {
    input: &'a str,
    cursor: usize,
    replaced: &'a mut bool,
}

impl<'a> SinglePassDisplayJsonParser<'a> {
    fn bytes(&self) -> &'a [u8] {
        self.input.as_bytes()
    }

    fn skip_ws(&mut self) {
        let bytes = self.bytes();
        while self.cursor < bytes.len()
            && matches!(bytes[self.cursor], b' ' | b'\t' | b'\r' | b'\n')
        {
            self.cursor += 1;
        }
    }

    fn scan_string_token(&mut self) -> Result<&'a str, serde_json::Error> {
        let bytes = self.bytes();
        let start = self.cursor;
        if bytes.get(start) != Some(&b'"') {
            return Err(
                serde_json::from_str::<serde_json::Value>(&self.input[start..]).unwrap_err(),
            );
        }
        let mut idx = start + 1;
        while idx < bytes.len() {
            match bytes[idx] {
                b'\\' => idx += 2,
                b'"' => {
                    self.cursor = idx + 1;
                    return Ok(&self.input[start..self.cursor]);
                }
                _ => idx += 1,
            }
        }
        Err(serde_json::from_str::<serde_json::Value>(&self.input[start..]).unwrap_err())
    }

    fn parse_display_string(&mut self) -> Result<String, serde_json::Error> {
        let token = self.scan_string_token()?;
        let string = JavaString::from_json_string(token)?;
        *self.replaced |= string.try_to_string().is_err();
        Ok(string.to_display_string())
    }

    fn parse_value(&mut self) -> Result<serde_json::Value, serde_json::Error> {
        self.skip_ws();
        let start = self.cursor;
        let Some(&byte) = self.bytes().get(start) else {
            return serde_json::from_str("");
        };
        match byte {
            b'"' => Ok(serde_json::Value::String(self.parse_display_string()?)),
            b'[' => {
                self.cursor += 1;
                self.skip_ws();
                let mut items = Vec::new();
                if self.bytes().get(self.cursor) == Some(&b']') {
                    self.cursor += 1;
                    return Ok(serde_json::Value::Array(items));
                }
                loop {
                    items.push(self.parse_value()?);
                    self.skip_ws();
                    match self.bytes().get(self.cursor) {
                        Some(&b',') => self.cursor += 1,
                        Some(&b']') => {
                            self.cursor += 1;
                            return Ok(serde_json::Value::Array(items));
                        }
                        _ => return serde_json::from_str(&self.input[self.cursor..]),
                    }
                }
            }
            b'{' => {
                self.cursor += 1;
                self.skip_ws();
                let mut map = serde_json::Map::new();
                if self.bytes().get(self.cursor) == Some(&b'}') {
                    self.cursor += 1;
                    return Ok(serde_json::Value::Object(map));
                }
                loop {
                    self.skip_ws();
                    let key = self.parse_display_string()?;
                    self.skip_ws();
                    if self.bytes().get(self.cursor) != Some(&b':') {
                        return serde_json::from_str(&self.input[self.cursor..]);
                    }
                    self.cursor += 1;
                    let value = self.parse_value()?;
                    map.insert(key, value);
                    self.skip_ws();
                    match self.bytes().get(self.cursor) {
                        Some(&b',') => self.cursor += 1,
                        Some(&b'}') => {
                            self.cursor += 1;
                            return Ok(serde_json::Value::Object(map));
                        }
                        _ => return serde_json::from_str(&self.input[self.cursor..]),
                    }
                }
            }
            _ => {
                let bytes = self.bytes();
                let mut end = start;
                while end < bytes.len()
                    && !matches!(
                        bytes[end],
                        b' ' | b'\t' | b'\r' | b'\n' | b',' | b']' | b'}'
                    )
                {
                    end += 1;
                }
                self.cursor = end;
                serde_json::from_str(&self.input[start..end])
            }
        }
    }
}

/// Java `Character.isLetterOrDigit(char)` over one UTF-16 code unit.
pub fn java_char_is_letter_or_digit(unit: u16) -> bool {
    let mut low = 0;
    let mut high = JAVA_LETTER_OR_DIGIT_RANGES.len() / 2;
    while low < high {
        let middle = low + (high - low) / 2;
        let start = JAVA_LETTER_OR_DIGIT_RANGES[middle * 2];
        let end = JAVA_LETTER_OR_DIGIT_RANGES[middle * 2 + 1];
        if unit < start {
            high = middle;
        } else if unit > end {
            low = middle + 1;
        } else {
            return true;
        }
    }
    false
}

// OpenJDK 25.0.3 `Character.isLetterOrDigit(char)` ranges. Minecraft 26.1's
// parser deliberately calls the `char`, not code-point, overload.
const JAVA_LETTER_OR_DIGIT_RANGES: &[u16] = &[
    0x0030, 0x0039, 0x0041, 0x005a, 0x0061, 0x007a, 0x00aa, 0x00aa, 0x00b5, 0x00b5, 0x00ba, 0x00ba,
    0x00c0, 0x00d6, 0x00d8, 0x00f6, 0x00f8, 0x02c1, 0x02c6, 0x02d1, 0x02e0, 0x02e4, 0x02ec, 0x02ec,
    0x02ee, 0x02ee, 0x0370, 0x0374, 0x0376, 0x0377, 0x037a, 0x037d, 0x037f, 0x037f, 0x0386, 0x0386,
    0x0388, 0x038a, 0x038c, 0x038c, 0x038e, 0x03a1, 0x03a3, 0x03f5, 0x03f7, 0x0481, 0x048a, 0x052f,
    0x0531, 0x0556, 0x0559, 0x0559, 0x0560, 0x0588, 0x05d0, 0x05ea, 0x05ef, 0x05f2, 0x0620, 0x064a,
    0x0660, 0x0669, 0x066e, 0x066f, 0x0671, 0x06d3, 0x06d5, 0x06d5, 0x06e5, 0x06e6, 0x06ee, 0x06fc,
    0x06ff, 0x06ff, 0x0710, 0x0710, 0x0712, 0x072f, 0x074d, 0x07a5, 0x07b1, 0x07b1, 0x07c0, 0x07ea,
    0x07f4, 0x07f5, 0x07fa, 0x07fa, 0x0800, 0x0815, 0x081a, 0x081a, 0x0824, 0x0824, 0x0828, 0x0828,
    0x0840, 0x0858, 0x0860, 0x086a, 0x0870, 0x0887, 0x0889, 0x088e, 0x08a0, 0x08c9, 0x0904, 0x0939,
    0x093d, 0x093d, 0x0950, 0x0950, 0x0958, 0x0961, 0x0966, 0x096f, 0x0971, 0x0980, 0x0985, 0x098c,
    0x098f, 0x0990, 0x0993, 0x09a8, 0x09aa, 0x09b0, 0x09b2, 0x09b2, 0x09b6, 0x09b9, 0x09bd, 0x09bd,
    0x09ce, 0x09ce, 0x09dc, 0x09dd, 0x09df, 0x09e1, 0x09e6, 0x09f1, 0x09fc, 0x09fc, 0x0a05, 0x0a0a,
    0x0a0f, 0x0a10, 0x0a13, 0x0a28, 0x0a2a, 0x0a30, 0x0a32, 0x0a33, 0x0a35, 0x0a36, 0x0a38, 0x0a39,
    0x0a59, 0x0a5c, 0x0a5e, 0x0a5e, 0x0a66, 0x0a6f, 0x0a72, 0x0a74, 0x0a85, 0x0a8d, 0x0a8f, 0x0a91,
    0x0a93, 0x0aa8, 0x0aaa, 0x0ab0, 0x0ab2, 0x0ab3, 0x0ab5, 0x0ab9, 0x0abd, 0x0abd, 0x0ad0, 0x0ad0,
    0x0ae0, 0x0ae1, 0x0ae6, 0x0aef, 0x0af9, 0x0af9, 0x0b05, 0x0b0c, 0x0b0f, 0x0b10, 0x0b13, 0x0b28,
    0x0b2a, 0x0b30, 0x0b32, 0x0b33, 0x0b35, 0x0b39, 0x0b3d, 0x0b3d, 0x0b5c, 0x0b5d, 0x0b5f, 0x0b61,
    0x0b66, 0x0b6f, 0x0b71, 0x0b71, 0x0b83, 0x0b83, 0x0b85, 0x0b8a, 0x0b8e, 0x0b90, 0x0b92, 0x0b95,
    0x0b99, 0x0b9a, 0x0b9c, 0x0b9c, 0x0b9e, 0x0b9f, 0x0ba3, 0x0ba4, 0x0ba8, 0x0baa, 0x0bae, 0x0bb9,
    0x0bd0, 0x0bd0, 0x0be6, 0x0bef, 0x0c05, 0x0c0c, 0x0c0e, 0x0c10, 0x0c12, 0x0c28, 0x0c2a, 0x0c39,
    0x0c3d, 0x0c3d, 0x0c58, 0x0c5a, 0x0c5d, 0x0c5d, 0x0c60, 0x0c61, 0x0c66, 0x0c6f, 0x0c80, 0x0c80,
    0x0c85, 0x0c8c, 0x0c8e, 0x0c90, 0x0c92, 0x0ca8, 0x0caa, 0x0cb3, 0x0cb5, 0x0cb9, 0x0cbd, 0x0cbd,
    0x0cdd, 0x0cde, 0x0ce0, 0x0ce1, 0x0ce6, 0x0cef, 0x0cf1, 0x0cf2, 0x0d04, 0x0d0c, 0x0d0e, 0x0d10,
    0x0d12, 0x0d3a, 0x0d3d, 0x0d3d, 0x0d4e, 0x0d4e, 0x0d54, 0x0d56, 0x0d5f, 0x0d61, 0x0d66, 0x0d6f,
    0x0d7a, 0x0d7f, 0x0d85, 0x0d96, 0x0d9a, 0x0db1, 0x0db3, 0x0dbb, 0x0dbd, 0x0dbd, 0x0dc0, 0x0dc6,
    0x0de6, 0x0def, 0x0e01, 0x0e30, 0x0e32, 0x0e33, 0x0e40, 0x0e46, 0x0e50, 0x0e59, 0x0e81, 0x0e82,
    0x0e84, 0x0e84, 0x0e86, 0x0e8a, 0x0e8c, 0x0ea3, 0x0ea5, 0x0ea5, 0x0ea7, 0x0eb0, 0x0eb2, 0x0eb3,
    0x0ebd, 0x0ebd, 0x0ec0, 0x0ec4, 0x0ec6, 0x0ec6, 0x0ed0, 0x0ed9, 0x0edc, 0x0edf, 0x0f00, 0x0f00,
    0x0f20, 0x0f29, 0x0f40, 0x0f47, 0x0f49, 0x0f6c, 0x0f88, 0x0f8c, 0x1000, 0x102a, 0x103f, 0x1049,
    0x1050, 0x1055, 0x105a, 0x105d, 0x1061, 0x1061, 0x1065, 0x1066, 0x106e, 0x1070, 0x1075, 0x1081,
    0x108e, 0x108e, 0x1090, 0x1099, 0x10a0, 0x10c5, 0x10c7, 0x10c7, 0x10cd, 0x10cd, 0x10d0, 0x10fa,
    0x10fc, 0x1248, 0x124a, 0x124d, 0x1250, 0x1256, 0x1258, 0x1258, 0x125a, 0x125d, 0x1260, 0x1288,
    0x128a, 0x128d, 0x1290, 0x12b0, 0x12b2, 0x12b5, 0x12b8, 0x12be, 0x12c0, 0x12c0, 0x12c2, 0x12c5,
    0x12c8, 0x12d6, 0x12d8, 0x1310, 0x1312, 0x1315, 0x1318, 0x135a, 0x1380, 0x138f, 0x13a0, 0x13f5,
    0x13f8, 0x13fd, 0x1401, 0x166c, 0x166f, 0x167f, 0x1681, 0x169a, 0x16a0, 0x16ea, 0x16f1, 0x16f8,
    0x1700, 0x1711, 0x171f, 0x1731, 0x1740, 0x1751, 0x1760, 0x176c, 0x176e, 0x1770, 0x1780, 0x17b3,
    0x17d7, 0x17d7, 0x17dc, 0x17dc, 0x17e0, 0x17e9, 0x1810, 0x1819, 0x1820, 0x1878, 0x1880, 0x1884,
    0x1887, 0x18a8, 0x18aa, 0x18aa, 0x18b0, 0x18f5, 0x1900, 0x191e, 0x1946, 0x196d, 0x1970, 0x1974,
    0x1980, 0x19ab, 0x19b0, 0x19c9, 0x19d0, 0x19d9, 0x1a00, 0x1a16, 0x1a20, 0x1a54, 0x1a80, 0x1a89,
    0x1a90, 0x1a99, 0x1aa7, 0x1aa7, 0x1b05, 0x1b33, 0x1b45, 0x1b4c, 0x1b50, 0x1b59, 0x1b83, 0x1ba0,
    0x1bae, 0x1be5, 0x1c00, 0x1c23, 0x1c40, 0x1c49, 0x1c4d, 0x1c7d, 0x1c80, 0x1c8a, 0x1c90, 0x1cba,
    0x1cbd, 0x1cbf, 0x1ce9, 0x1cec, 0x1cee, 0x1cf3, 0x1cf5, 0x1cf6, 0x1cfa, 0x1cfa, 0x1d00, 0x1dbf,
    0x1e00, 0x1f15, 0x1f18, 0x1f1d, 0x1f20, 0x1f45, 0x1f48, 0x1f4d, 0x1f50, 0x1f57, 0x1f59, 0x1f59,
    0x1f5b, 0x1f5b, 0x1f5d, 0x1f5d, 0x1f5f, 0x1f7d, 0x1f80, 0x1fb4, 0x1fb6, 0x1fbc, 0x1fbe, 0x1fbe,
    0x1fc2, 0x1fc4, 0x1fc6, 0x1fcc, 0x1fd0, 0x1fd3, 0x1fd6, 0x1fdb, 0x1fe0, 0x1fec, 0x1ff2, 0x1ff4,
    0x1ff6, 0x1ffc, 0x2071, 0x2071, 0x207f, 0x207f, 0x2090, 0x209c, 0x2102, 0x2102, 0x2107, 0x2107,
    0x210a, 0x2113, 0x2115, 0x2115, 0x2119, 0x211d, 0x2124, 0x2124, 0x2126, 0x2126, 0x2128, 0x2128,
    0x212a, 0x212d, 0x212f, 0x2139, 0x213c, 0x213f, 0x2145, 0x2149, 0x214e, 0x214e, 0x2183, 0x2184,
    0x2c00, 0x2ce4, 0x2ceb, 0x2cee, 0x2cf2, 0x2cf3, 0x2d00, 0x2d25, 0x2d27, 0x2d27, 0x2d2d, 0x2d2d,
    0x2d30, 0x2d67, 0x2d6f, 0x2d6f, 0x2d80, 0x2d96, 0x2da0, 0x2da6, 0x2da8, 0x2dae, 0x2db0, 0x2db6,
    0x2db8, 0x2dbe, 0x2dc0, 0x2dc6, 0x2dc8, 0x2dce, 0x2dd0, 0x2dd6, 0x2dd8, 0x2dde, 0x2e2f, 0x2e2f,
    0x3005, 0x3006, 0x3031, 0x3035, 0x303b, 0x303c, 0x3041, 0x3096, 0x309d, 0x309f, 0x30a1, 0x30fa,
    0x30fc, 0x30ff, 0x3105, 0x312f, 0x3131, 0x318e, 0x31a0, 0x31bf, 0x31f0, 0x31ff, 0x3400, 0x4dbf,
    0x4e00, 0xa48c, 0xa4d0, 0xa4fd, 0xa500, 0xa60c, 0xa610, 0xa62b, 0xa640, 0xa66e, 0xa67f, 0xa69d,
    0xa6a0, 0xa6e5, 0xa717, 0xa71f, 0xa722, 0xa788, 0xa78b, 0xa7cd, 0xa7d0, 0xa7d1, 0xa7d3, 0xa7d3,
    0xa7d5, 0xa7dc, 0xa7f2, 0xa801, 0xa803, 0xa805, 0xa807, 0xa80a, 0xa80c, 0xa822, 0xa840, 0xa873,
    0xa882, 0xa8b3, 0xa8d0, 0xa8d9, 0xa8f2, 0xa8f7, 0xa8fb, 0xa8fb, 0xa8fd, 0xa8fe, 0xa900, 0xa925,
    0xa930, 0xa946, 0xa960, 0xa97c, 0xa984, 0xa9b2, 0xa9cf, 0xa9d9, 0xa9e0, 0xa9e4, 0xa9e6, 0xa9fe,
    0xaa00, 0xaa28, 0xaa40, 0xaa42, 0xaa44, 0xaa4b, 0xaa50, 0xaa59, 0xaa60, 0xaa76, 0xaa7a, 0xaa7a,
    0xaa7e, 0xaaaf, 0xaab1, 0xaab1, 0xaab5, 0xaab6, 0xaab9, 0xaabd, 0xaac0, 0xaac0, 0xaac2, 0xaac2,
    0xaadb, 0xaadd, 0xaae0, 0xaaea, 0xaaf2, 0xaaf4, 0xab01, 0xab06, 0xab09, 0xab0e, 0xab11, 0xab16,
    0xab20, 0xab26, 0xab28, 0xab2e, 0xab30, 0xab5a, 0xab5c, 0xab69, 0xab70, 0xabe2, 0xabf0, 0xabf9,
    0xac00, 0xd7a3, 0xd7b0, 0xd7c6, 0xd7cb, 0xd7fb, 0xf900, 0xfa6d, 0xfa70, 0xfad9, 0xfb00, 0xfb06,
    0xfb13, 0xfb17, 0xfb1d, 0xfb1d, 0xfb1f, 0xfb28, 0xfb2a, 0xfb36, 0xfb38, 0xfb3c, 0xfb3e, 0xfb3e,
    0xfb40, 0xfb41, 0xfb43, 0xfb44, 0xfb46, 0xfbb1, 0xfbd3, 0xfd3d, 0xfd50, 0xfd8f, 0xfd92, 0xfdc7,
    0xfdf0, 0xfdfb, 0xfe70, 0xfe74, 0xfe76, 0xfefc, 0xff10, 0xff19, 0xff21, 0xff3a, 0xff41, 0xff5a,
    0xff66, 0xffbe, 0xffc2, 0xffc7, 0xffca, 0xffcf, 0xffd2, 0xffd7, 0xffda, 0xffdc,
];

/// JDK `String.getBytes(UTF_8)` replaces each unpaired surrogate with `?`.
pub(crate) fn encode_jdk_utf8(units: &[u16], mut output: impl FnMut(&[u8])) {
    let mut buffer = [0; 4];
    for scalar in char::decode_utf16(units.iter().copied()) {
        output(scalar.unwrap_or('?').encode_utf8(&mut buffer).as_bytes());
    }
}

#[inline]
pub fn java_character_is_whitespace(unit: u16) -> bool {
    matches!(unit, 0x0009..=0x000d | 0x001c..=0x0020 | 0x1680 | 0x2000..=0x2006
        | 0x2008..=0x200a | 0x2028 | 0x2029 | 0x205f | 0x3000)
}

pub fn java_string_hash_code(text: &str) -> i32 {
    text.encode_utf16().fold(0i32, |hash, unit| {
        hash.wrapping_mul(31).wrapping_add(i32::from(unit))
    })
}

pub fn code_of_conduct_text_hash(text: &str) -> i32 {
    java_string_hash_code(text)
}

#[cfg(test)]
mod tests {
    use super::{to_display_json_value, JavaString};

    #[test]
    fn borrowed_utf8_conversion_preserves_valid_text_and_rejects_raw_surrogates() {
        for text in [
            "",
            "short ASCII",
            &"x".repeat(128),
            &"x".repeat(129),
            "text 🦀",
        ] {
            let value = JavaString::from(text);
            assert_eq!(
                JavaString::with_utf8_units(value.units(), |raw| raw == text),
                Some(true)
            );
            assert_eq!(value.with_utf8(|raw| raw == text), Some(true));
        }
        assert_eq!(
            JavaString::with_utf8_units(&[0xd800], |_| panic!("invalid UTF-16")),
            None::<()>
        );
        // OpenJDK 25.0.3 String.getBytes(StandardCharsets.UTF_8).
        let cases: &[(&[u16], &[u8])] = &[
            (&[0xd800], b"?"),
            (&[0xd801], b"?"),
            (&[0xfffd], &[0xef, 0xbf, 0xbd]),
            (&[0xd800, 0xd800, 0xdc00], &[0x3f, 0xf0, 0x90, 0x80, 0x80]),
            (&[0xd800, 65], b"?A"),
            (&[0xd83d, 0xde00], &[0xf0, 0x9f, 0x98, 0x80]),
            (&[65, 83, 67, 73, 73], b"ASCII"),
            (&[0xdc00], b"?"),
            (&[0xd800, 0x100], &[0x3f, 0xc4, 0x80]),
            (&[0xd800, 0x20ac], &[0x3f, 0xe2, 0x82, 0xac]),
            (&[], b""),
        ];
        for (units, expected) in cases {
            assert_eq!(JavaString::from(*units).to_jdk_utf8(), *expected);
        }
    }

    #[test]
    fn lowercase_keeps_raw_units_and_word_boundaries() {
        assert!(JavaString::default().to_lowercase_root().is_empty());
        assert_eq!(
            JavaString::from("AZ az 09 @[_\u{7f}")
                .to_lowercase_root()
                .units(),
            JavaString::from("az az 09 @[_\u{7f}").units()
        );
        let text = JavaString::from(vec![0x41, 0x03a3, 0xd800, 0x42, 0x0130, 0xd801]);
        assert_eq!(
            text.to_lowercase_root().units(),
            &[0x61, 0x03c2, 0xd800, 0x62, 0x69, 0x0307, 0xd801]
        );
    }

    #[test]
    fn json_strings_preserve_utf16_units() {
        for units in [
            vec![0xd800],
            vec![0xd801],
            vec![0xfffd],
            vec![b'a' as u16, 0xd800, 0xdc00, 0xd801, 0xfffd],
            "text 🦀 \\uD800\n\"".encode_utf16().collect(),
        ] {
            let value = JavaString::from(units);
            let json = serde_json::to_string(&value).unwrap();
            assert!(json.starts_with('"') && json.ends_with('"'));
            assert_eq!(serde_json::from_str::<JavaString>(&json).unwrap(), value);
            assert_eq!(JavaString::from_json_string(&json).unwrap(), value);
        }
        assert_eq!(
            serde_json::from_value::<JavaString>(serde_json::json!("plain 🦀 \\uD800")).unwrap(),
            JavaString::from("plain 🦀 \\uD800")
        );
        assert!(serde_json::from_str::<JavaString>("[237,160,128]").is_err());
        assert!(serde_json::from_slice::<JavaString>(b"\"\xc0\x80\"").is_err());
        assert!(serde_json::from_str::<JavaString>(r#""\q""#).is_err());

        #[derive(serde::Serialize)]
        struct DisplayInput {
            text: JavaString,
            literal: &'static str,
            shape: serde_json::Value,
        }
        let input = DisplayInput {
            text: vec![0xd800, b'x' as u16, 0xd801, 0xfffd].into(),
            literal: "\\uD800",
            shape: serde_json::json!([null, true, 3, {"nested": "🦀"}]),
        };
        assert!(serde_json::to_value(&input).is_err());
        assert_eq!(
            to_display_json_value(&input).unwrap(),
            serde_json::json!({
                "text": "�x��",
                "literal": "\\uD800",
                "shape": input.shape,
            })
        );
        let ordinary = JavaString::from("ordinary 🦀");
        assert_eq!(
            to_display_json_value(&ordinary).unwrap(),
            serde_json::to_value(&ordinary).unwrap()
        );

        #[derive(serde::Serialize)]
        #[serde(untagged)]
        enum Nested {
            Text(JavaString),
            Array(Vec<Nested>),
        }
        let mut nested = Nested::Text(vec![0xd800].into());
        for _ in 0..130 {
            nested = Nested::Array(vec![nested]);
        }
        let mut projected = to_display_json_value(&nested).unwrap();
        for _ in 0..130 {
            projected = projected.as_array_mut().unwrap().pop().unwrap();
        }
        assert_eq!(projected, serde_json::json!("�"));

        struct Reject;
        impl serde::Serialize for Reject {
            fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
                Err(serde::ser::Error::custom("rejected input"))
            }
        }
        assert_eq!(
            to_display_json_value(&Reject).unwrap_err().to_string(),
            "rejected input"
        );
    }

    #[test]
    fn minecraft_blank_uses_character_whitespace_and_space_char() {
        for units in [
            vec![],
            vec![0x20],
            vec![0xa0],
            vec![0x2007],
            vec![0x202f],
            vec![0x3000],
        ] {
            assert!(JavaString::minecraft_units_are_blank(&units));
        }
        for units in [vec![0x85], vec![0xd800], vec![0xfffd], vec![0x20, 0x85]] {
            assert!(!JavaString::minecraft_units_are_blank(&units));
        }
    }
}
