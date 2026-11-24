//! The DFU primitive codecs the particle option codecs decode NBT with. Only
//! acceptance matters: a refusal carries a short message, not DFU's wording.

use bbb_protocol::nbt_summary::{codec_double_value, codec_float_value, codec_int_value};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

pub(super) type Decoded<T> = Result<T, String>;

/// `EitherCodec.decode`: the second codec only runs when the first refuses.
pub(super) fn either<T>(first: Decoded<T>, second: impl FnOnce() -> Decoded<T>) -> Decoded<T> {
    first.or_else(|_| second())
}

/// `Codec.INT`.
pub(super) fn int(value: &NbtSummaryValue) -> Decoded<i32> {
    codec_int_value(value).ok_or_else(|| "not a number".into())
}

/// `Codec.FLOAT`.
pub(super) fn float(value: &NbtSummaryValue) -> Decoded<f32> {
    codec_float_value(value).ok_or_else(|| "not a number".into())
}

/// `Codec.DOUBLE`.
pub(super) fn double(value: &NbtSummaryValue) -> Decoded<f64> {
    codec_double_value(value).ok_or_else(|| "not a number".into())
}

/// `Codec.STRING` over `NbtOps.getStringValue` (NbtOps.java:101-103).
pub(super) fn string(value: &NbtSummaryValue) -> Decoded<String> {
    match value {
        NbtSummaryValue::String(value) => {
            value.try_to_string().map_err(|_| "not Unicode text".into())
        }
        _ => Err("not a string".into()),
    }
}

/// `ListCodec` over `NbtOps.getList` (NbtOps.java:238-240) narrowed by
/// `Util.fixedSize` (Util.java:919-926). Checks collection length before
/// decoding elements so oversized NBT arrays cannot trigger allocation blowup.
pub(super) fn fixed_list<T>(
    value: &NbtSummaryValue,
    size: usize,
    element: impl Fn(&NbtSummaryValue) -> Decoded<T>,
) -> Decoded<Vec<T>> {
    let len = match value {
        NbtSummaryValue::List(items) => items.len(),
        NbtSummaryValue::ByteArray(items) => items.len(),
        NbtSummaryValue::IntArray(items) => items.len(),
        NbtSummaryValue::LongArray(items) => items.len(),
        _ => return Err("not a list".into()),
    };
    if len != size {
        return Err(format!("expected {size} elements"));
    }
    let mut values = Vec::with_capacity(size);
    match value {
        NbtSummaryValue::List(items) => {
            for item in items {
                values.push(element(item)?);
            }
        }
        NbtSummaryValue::ByteArray(items) => {
            for &item in items {
                values.push(element(&NbtSummaryValue::Byte(item))?);
            }
        }
        NbtSummaryValue::IntArray(items) => {
            for &item in items {
                values.push(element(&NbtSummaryValue::Int(item))?);
            }
        }
        NbtSummaryValue::LongArray(items) => {
            for &item in items {
                values.push(element(&NbtSummaryValue::Long(item))?);
            }
        }
        _ => unreachable!(),
    }
    Ok(values)
}

/// `Codec.INT_STREAM` over `NbtOps.getIntStream` (NbtOps.java:253-255)
/// narrowed by `Util.fixedSize` (Util.java:899-907); any collection of numbers
/// is read, not only an int array.
pub(super) fn fixed_ints(value: &NbtSummaryValue, size: usize) -> Decoded<Vec<i32>> {
    fixed_list(value, size, int)
}

/// A compound as `NbtOps.getMap` (NbtOps.java:194-218) exposes it.
#[derive(Clone, Copy)]
pub(super) struct MapLike<'a> {
    entries: &'a [NbtSummaryEntry],
}

pub(super) fn map(value: &NbtSummaryValue) -> Decoded<MapLike<'_>> {
    match value {
        NbtSummaryValue::Compound(entries) => Ok(MapLike { entries }),
        _ => Err("not a compound".into()),
    }
}

impl<'a> MapLike<'a> {
    pub fn entries(&self) -> &'a [NbtSummaryEntry] {
        self.entries
    }

    pub fn get(&self, name: &str) -> Option<&'a NbtSummaryValue> {
        self.entries
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| &entry.value)
    }

    pub fn field<T>(
        &self,
        name: &str,
        codec: impl FnOnce(&'a NbtSummaryValue) -> Decoded<T>,
    ) -> Decoded<T> {
        codec(self.get(name).ok_or_else(|| format!("missing {name}"))?)
    }

    /// `optionalFieldOf(name, default)`: a present value must still decode.
    pub fn optional_field<T>(
        &self,
        name: &str,
        default: T,
        codec: impl FnOnce(&'a NbtSummaryValue) -> Decoded<T>,
    ) -> Decoded<T> {
        self.get(name).map_or(Ok(default), codec)
    }
}

/// `Identifier.CODEC` (Identifier.java:18).
fn identifier(value: &NbtSummaryValue) -> Decoded<String> {
    let raw = string(value)?;
    try_parse_identifier(&raw).ok_or_else(|| format!("invalid identifier: {raw}"))
}

/// `Identifier.tryParse` (Identifier.java:51-53) over `bySeparator`
/// (`:59-72`): the namespace is checked only when the text names one.
pub(super) fn try_parse_identifier(raw: &str) -> Option<String> {
    let (namespace, path) = match raw.split_once(':') {
        Some(("", path)) => ("minecraft", path),
        Some((namespace, path)) if valid_namespace(namespace) => (namespace, path),
        Some(_) => return None,
        None => ("minecraft", raw),
    };
    valid_path(path).then(|| format!("{namespace}:{path}"))
}

/// `Identifier.isValidNamespace` (Identifier.java:230-242).
fn valid_namespace(namespace: &str) -> bool {
    namespace != ".."
        && namespace.bytes().all(|byte| {
            byte.is_ascii_digit() || byte.is_ascii_lowercase() || matches!(byte, b'_' | b'-' | b'.')
        })
}

/// `Identifier.isValidPath` (Identifier.java:220-228).
fn valid_path(path: &str) -> bool {
    path.bytes().all(|byte| {
        byte.is_ascii_digit()
            || byte.is_ascii_lowercase()
            || matches!(byte, b'_' | b'-' | b'/' | b'.')
    })
}

/// `Registry.byNameCodec` (Registry.java:29-46).
pub(super) fn registry_element<T>(
    value: &NbtSummaryValue,
    lookup: impl FnOnce(&str) -> Option<T>,
) -> Decoded<T> {
    let id = identifier(value)?;
    lookup(&id).ok_or_else(|| format!("unknown id: {id}"))
}
