//! Readers over a loot document, whether it came from datapack JSON or from
//! NBT (an inline command argument or saved entity data).

use bbb_protocol::{JavaString, NbtSummaryEntry, NbtSummaryValue};
use bbb_registry_codec::{RegistryElementSource, RegistryElementValueRef, RegistryJsonValue};

pub use crate::worldgen::json::{fail, At, DecodeError, Result};

pub type V<'a> = RegistryElementValueRef<'a>;

/// One record's members. A JSON `null` member reads as absent.
#[derive(Clone, Copy)]
pub struct Obj<'a>(V<'a>);

impl<'a> Obj<'a> {
    pub fn of(value: V<'a>) -> Result<Self> {
        if value.is_compound() {
            Ok(Self(value))
        } else {
            fail("expected an object")
        }
    }

    pub fn get(&self, name: &str) -> Option<V<'a>> {
        self.0.field(name)
    }

    pub fn has(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    pub fn req<T>(&self, name: &str, decode: impl FnOnce(V<'a>) -> Result<T>) -> Result<T> {
        match self.get(name) {
            Some(value) => decode(value).at(name),
            None => fail("missing required field").at(name),
        }
    }

    pub fn opt<T>(&self, name: &str, decode: impl FnOnce(V<'a>) -> Result<T>) -> Result<Option<T>> {
        self.get(name).map(decode).transpose().at(name)
    }

    pub fn or<T>(
        &self,
        name: &str,
        default: T,
        decode: impl FnOnce(V<'a>) -> Result<T>,
    ) -> Result<T> {
        Ok(self.opt(name, decode)?.unwrap_or(default))
    }

    /// `fieldOf(name).orElse(default)`: an undecodable value is the default too.
    pub fn or_else<T>(&self, name: &str, default: T, decode: impl FnOnce(V<'a>) -> Result<T>) -> T {
        self.get(name)
            .and_then(|value| decode(value).ok())
            .unwrap_or(default)
    }

    /// The members in source order; a repeated NBT key keeps its last value.
    pub fn entries(&self) -> Vec<(&'a JavaString, V<'a>)> {
        entries(self.0).unwrap_or_default()
    }

    pub fn keys(&self) -> impl Iterator<Item = &'a JavaString> {
        self.entries().into_iter().map(|(key, _)| key)
    }
}

pub fn entries(value: V<'_>) -> Option<Vec<(&'_ JavaString, V<'_>)>> {
    let mut out: Vec<(&'_ JavaString, V<'_>)> = Vec::new();
    for index in 0..value.map_len()? {
        let (key, field) = value.map_entry_java_string_ref(index)?;
        if let Some((_, previous)) = out.iter_mut().find(|(name, _)| *name == key) {
            *previous = field;
        } else {
            out.push((key, field));
        }
    }
    Some(out)
}

pub fn int(value: V<'_>) -> Result<i32> {
    value
        .codec_i32()
        .map_or_else(|| fail("expected an int"), Ok)
}

pub fn long(value: V<'_>) -> Result<i64> {
    value
        .codec_i64()
        .map_or_else(|| fail("expected a long"), Ok)
}

pub fn float(value: V<'_>) -> Result<f32> {
    value
        .codec_f32()
        .map_or_else(|| fail("expected a float"), Ok)
}

pub fn double(value: V<'_>) -> Result<f64> {
    value
        .codec_f64()
        .map_or_else(|| fail("expected a double"), Ok)
}

pub fn boolean(value: V<'_>) -> Result<bool> {
    value
        .as_bool()
        .map_or_else(|| fail("expected a boolean"), Ok)
}

pub fn string(value: V<'_>) -> Result<String> {
    value
        .utf8_string()
        .map_or_else(|| fail("expected Unicode text"), Ok)
}

/// `Identifier.CODEC`: the default namespace is `minecraft`.
pub fn id(value: V<'_>) -> Result<String> {
    let raw = string(value)?;
    bbb_protocol::normalize_resource_location(raw.clone())
        .or_else(|_| fail(format!("invalid identifier `{raw}`")))
}

pub fn is_number(value: V<'_>) -> bool {
    value.codec_f64().is_some()
}

pub fn list<'a, T>(value: V<'a>, mut decode: impl FnMut(V<'a>) -> Result<T>) -> Result<Vec<T>> {
    let Some(len) = value.list_len() else {
        return fail("expected a list");
    };
    (0..len)
        .map(|index| decode(value.list_element(index).expect("in range")).at_index(index))
        .collect()
}

pub fn source(value: V<'_>) -> RegistryElementSource {
    match value {
        V::Json(_) => RegistryElementSource::ServerDatapackJson,
        _ => RegistryElementSource::NetworkNbt,
    }
}

/// The NBT `JsonOps.convertTo(NbtOps)` builds from the value.
pub fn to_nbt(value: V<'_>) -> Result<NbtSummaryValue> {
    Ok(match value {
        V::Nbt(value) => value.clone(),
        V::Json(value) => bbb_registry_codec::json_to_nbt(value).or_else(fail)?,
        V::NbtByte(value) => NbtSummaryValue::Byte(value),
        V::NbtInt(value) => NbtSummaryValue::Int(value),
        V::NbtLong(value) => NbtSummaryValue::Long(value),
    })
}

/// `TagParser.LENIENT_CODEC`: an SNBT string or a compound.
pub fn custom_data_tag(value: V<'_>) -> Result<Vec<NbtSummaryEntry>> {
    let tag = match value.java_string_ref() {
        Some(text) => bbb_protocol::parse_snbt_compound_utf16(text.units())
            .or_else(|error| fail(error.to_string()))?,
        None => to_nbt(value)?,
    };
    match tag {
        NbtSummaryValue::Compound(fields) => Ok(fields),
        _ => fail("expected a compound tag"),
    }
}

/// An owned document in the source's own representation, for records a
/// function assembles from its fields before handing them to a component codec.
pub enum Owned {
    Json(RegistryJsonValue),
    Nbt(NbtSummaryValue),
}

impl Owned {
    pub fn of(value: V<'_>) -> Self {
        match value {
            V::Json(value) => Self::Json(value.clone()),
            other => Self::Nbt(to_nbt(other).expect("NBT converts to itself")),
        }
    }

    pub fn string(like: V<'_>, value: &str) -> Self {
        match like {
            V::Json(_) => Self::Json(RegistryJsonValue::String(value.into())),
            _ => Self::Nbt(NbtSummaryValue::String(value.into())),
        }
    }

    pub fn boolean(like: V<'_>, value: bool) -> Self {
        match like {
            V::Json(_) => Self::Json(RegistryJsonValue::Bool(value)),
            _ => Self::Nbt(NbtSummaryValue::Byte(i8::from(value))),
        }
    }

    pub fn record(like: V<'_>, fields: Vec<(&str, Owned)>) -> Self {
        match like {
            V::Json(_) => Self::Json(RegistryJsonValue::Object(
                fields
                    .into_iter()
                    .map(|(name, value)| match value {
                        Owned::Json(value) => (name.into(), value),
                        Owned::Nbt(_) => unreachable!("one source per record"),
                    })
                    .collect(),
            )),
            _ => Self::Nbt(NbtSummaryValue::Compound(
                fields
                    .into_iter()
                    .map(|(name, value)| match value {
                        Owned::Nbt(value) => NbtSummaryEntry {
                            name: name.into(),
                            value,
                        },
                        Owned::Json(_) => unreachable!("one source per record"),
                    })
                    .collect(),
            )),
        }
    }

    pub fn view(&self) -> V<'_> {
        match self {
            Self::Json(value) => V::Json(value),
            Self::Nbt(value) => V::Nbt(value),
        }
    }
}

/// A `serde_json` document re-read with its number lexemes, as datapack files are.
pub fn registry_json(value: &serde_json::Value) -> Result<RegistryJsonValue> {
    let bytes = serde_json::to_vec(value).or_else(|error| fail(error.to_string()))?;
    bbb_registry_codec::parse_registry_json(&bytes).or_else(|error| fail(error.to_string()))
}

/// `ComponentSerialization.CODEC`.
pub fn text_parameter(value: V<'_>) -> Result<NbtSummaryValue> {
    text_parameter_with_content_registry(
        value,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

/// Content-registry-aware form of [`text_parameter`].
pub fn text_parameter_with_content_registry(
    value: V<'_>,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<NbtSummaryValue> {
    bbb_registry_codec::decode_loot_text_parameter_source_with_content_registry(
        value,
        source(value),
        Default::default(),
        content_registry,
    )
    .or_else(|error| fail(error.to_string()))
}
