//! Readers over source-aware datapack JSON shared by the worldgen decoders.
//!
//! Numbers keep their source lexeme so `Codec.FLOAT` narrows the lexeme
//! directly, as Gson's `LazilyParsedNumber` does. A JSON `null` member reads as
//! absent, matching `JsonOps.getMap(...).get(field)`.

use std::fmt;

use bbb_protocol::java_number::{json_number_i32, json_number_i64};
use bbb_registry_codec::RegistryJsonValue as Json;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeError {
    pub path: String,
    pub message: String,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.path.is_empty() {
            f.write_str(&self.message)
        } else {
            write!(f, "{}: {}", self.path, self.message)
        }
    }
}

impl std::error::Error for DecodeError {}

impl From<bbb_registry_codec::RegistryCodecError> for DecodeError {
    fn from(error: bbb_registry_codec::RegistryCodecError) -> Self {
        Self {
            path: error.field,
            message: error.reason,
        }
    }
}

pub type Result<T> = std::result::Result<T, DecodeError>;

pub fn fail<T>(message: impl Into<String>) -> Result<T> {
    Err(DecodeError {
        path: String::new(),
        message: message.into(),
    })
}

/// Prefixes the failing field onto an error path while it unwinds.
pub trait At<T> {
    fn at(self, segment: &(impl fmt::Display + ?Sized)) -> Result<T>;
    fn at_index(self, index: usize) -> Result<T>;
}

impl<T> At<T> for Result<T> {
    fn at(self, segment: &(impl fmt::Display + ?Sized)) -> Result<T> {
        self.map_err(|mut error| {
            error.path = if error.path.is_empty() {
                segment.to_string()
            } else if error.path.starts_with('[') {
                format!("{segment}{}", error.path)
            } else {
                format!("{segment}.{}", error.path)
            };
            error
        })
    }

    fn at_index(self, index: usize) -> Result<T> {
        self.at(&format!("[{index}]"))
    }
}

/// One JSON object's members.
#[derive(Clone, Copy)]
pub struct Obj<'a> {
    entries: &'a [(bbb_protocol::JavaString, Json)],
}

impl<'a> Obj<'a> {
    pub fn of(value: &'a Json) -> Result<Self> {
        match value {
            Json::Object(entries) => Ok(Self { entries }),
            _ => fail("expected an object"),
        }
    }

    pub fn get(&self, name: &str) -> Option<&'a Json> {
        self.entries
            .iter()
            .find(|(entry, _)| entry == name)
            .map(|(_, value)| value)
            .filter(|value| !matches!(value, Json::Null))
    }

    pub fn has(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    pub fn req<T>(&self, name: &str, decode: impl FnOnce(&'a Json) -> Result<T>) -> Result<T> {
        match self.get(name) {
            Some(value) => decode(value).at(name),
            None => fail("missing required field").at(name),
        }
    }

    pub fn opt<T>(
        &self,
        name: &str,
        decode: impl FnOnce(&'a Json) -> Result<T>,
    ) -> Result<Option<T>> {
        self.get(name).map(decode).transpose().at(name)
    }

    pub fn or<T>(
        &self,
        name: &str,
        default: T,
        decode: impl FnOnce(&'a Json) -> Result<T>,
    ) -> Result<T> {
        Ok(self.opt(name, decode)?.unwrap_or(default))
    }

    /// `fieldOf(name).orElse(default)`: an absent or undecodable value reads
    /// as the default.
    pub fn or_else<T>(
        &self,
        name: &str,
        default: T,
        decode: impl FnOnce(&'a Json) -> Result<T>,
    ) -> T {
        self.get(name)
            .and_then(|value| decode(value).ok())
            .unwrap_or(default)
    }

    /// The normalized resource id in `field` (`type` for most dispatch codecs).
    pub fn type_id(&self, field: &str) -> Result<String> {
        self.req(field, id)
    }
}

pub fn int(value: &Json) -> Result<i32> {
    match value {
        Json::Number(lexeme) => match json_number_i32(lexeme) {
            Some(value) => Ok(value),
            None => fail("expected an int"),
        },
        _ => fail("expected a number"),
    }
}

/// `Codec.intRange(min, max)`.
pub fn int_in(value: &Json, min: i32, max: i32) -> Result<i32> {
    let value = int(value)?;
    if (min..=max).contains(&value) {
        Ok(value)
    } else {
        fail(format!("value {value} outside of range [{min}:{max}]"))
    }
}

pub fn long(value: &Json) -> Result<i64> {
    match value {
        Json::Number(lexeme) => match json_number_i64(lexeme) {
            Some(value) => Ok(value),
            None => fail("expected a long"),
        },
        _ => fail("expected a number"),
    }
}

pub fn float(value: &Json) -> Result<f32> {
    match value {
        Json::Number(lexeme) => lexeme.parse().or_else(|_| fail("expected a float")),
        _ => fail("expected a number"),
    }
}

/// `Codec.floatRange(min, max)`.
pub fn float_in(value: &Json, min: f32, max: f32) -> Result<f32> {
    let value = float(value)?;
    if value >= min && value <= max {
        Ok(value)
    } else {
        fail(format!("value {value} outside of range [{min}:{max}]"))
    }
}

pub fn double(value: &Json) -> Result<f64> {
    match value {
        Json::Number(lexeme) => lexeme.parse().or_else(|_| fail("expected a double")),
        _ => fail("expected a number"),
    }
}

/// `Codec.doubleRange(min, max)`.
pub fn double_in(value: &Json, min: f64, max: f64) -> Result<f64> {
    let value = double(value)?;
    if value >= min && value <= max {
        Ok(value)
    } else {
        fail(format!("value {value} outside of range [{min}:{max}]"))
    }
}

pub fn boolean(value: &Json) -> Result<bool> {
    match value {
        Json::Bool(value) => Ok(*value),
        _ => fail("expected a boolean"),
    }
}

pub fn string(value: &Json) -> Result<String> {
    match value {
        Json::String(value) => value
            .try_to_string()
            .or_else(|_| fail("expected Unicode text")),
        _ => fail("expected a string"),
    }
}

/// `Identifier.CODEC`: the default namespace is `minecraft`.
pub fn id(value: &Json) -> Result<String> {
    let raw = string(value)?;
    bbb_protocol::normalize_resource_location(raw.clone())
        .or_else(|_| fail(format!("invalid identifier `{raw}`")))
}

pub fn list<'a, T>(
    value: &'a Json,
    mut decode: impl FnMut(&'a Json) -> Result<T>,
) -> Result<Vec<T>> {
    match value {
        Json::Array(values) => values
            .iter()
            .enumerate()
            .map(|(index, value)| decode(value).at_index(index))
            .collect(),
        _ => fail("expected a list"),
    }
}

/// Checks the source list's resource budget before decoding or allocating its elements.
pub fn bounded_list<'a, T>(
    value: &'a Json,
    min_len: usize,
    max_len: usize,
    decode: impl FnMut(&'a Json) -> Result<T>,
) -> Result<Vec<T>> {
    let Json::Array(values) = value else {
        return fail("expected a list");
    };
    if !(min_len..=max_len).contains(&values.len()) {
        return fail(format!("list must contain {min_len}..={max_len} values"));
    }
    list(value, decode)
}

pub fn nonempty_list<'a, T>(
    value: &'a Json,
    decode: impl FnMut(&'a Json) -> Result<T>,
) -> Result<Vec<T>> {
    let values = list(value, decode)?;
    if values.is_empty() {
        return fail("list must not be empty");
    }
    Ok(values)
}

/// A value that is either one element or a list of them, as
/// `ExtraCodecs.compactListCodec` and the holder-set codecs accept.
pub fn one_or_list<'a, T>(
    value: &'a Json,
    mut decode: impl FnMut(&'a Json) -> Result<T>,
) -> Result<Vec<T>> {
    match value {
        Json::Array(_) => list(value, decode),
        _ => Ok(vec![decode(value)?]),
    }
}
