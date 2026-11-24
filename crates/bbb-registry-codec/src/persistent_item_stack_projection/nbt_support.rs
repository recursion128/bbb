//! NBT field access and the Vanilla value ranges both projection
//! directions share.

use std::borrow::Cow;

use bbb_protocol::nbt_summary::canonical_float_tag_bits;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::error::{invalid_persistent_component, Result};

// Boxed Float comparison places -0.0 below a +0.0 lower bound.
pub(super) fn checked_float(
    type_id: i32,
    resource_id: &'static str,
    value: f32,
    min: f32,
    max: f32,
    what: &str,
) -> Result<f32> {
    if !(value >= min && value <= max) || (min == 0.0 && value.to_bits() == (-0.0_f32).to_bits()) {
        return Err(invalid_persistent_component(
            type_id,
            resource_id,
            format!("{what} {} is outside {min}..={max}", value),
        ));
    }
    Ok(value)
}

/// One `Codec.FLOAT` value as `NbtOps` writes it: a `FloatTag`, whose
/// `valueOf` folds `-0.0` onto `0.0`.
pub(super) fn persistent_float_nbt(bits: u32) -> NbtSummaryValue {
    NbtSummaryValue::Float(canonical_float_tag_bits(f32::from_bits(bits)))
}

/// One `Codec.BOOL` record field, which `NbtOps` writes as a `ByteTag`
/// and `optionalFieldOf` drops while it holds its default.
pub(super) fn persistent_flag_nbt(value: bool) -> NbtSummaryValue {
    NbtSummaryValue::Byte(if value { 1 } else { 0 })
}

/// `RegistryCodecs.homogeneousList(registry)` is a
/// `HolderSetCodec` whose registry-aware form is `Codec.either` of
/// `TagKey.hashedCodec` -- the `"#"`-prefixed tag id --
/// and `ExtraCodecs.compactListCodec`, the bare element for a one-holder set and
/// a list otherwise.
pub(super) fn persistent_compact_holder_set_nbt<T: AsRef<str>>(
    type_id: i32,
    resource_id: &'static str,
    tag: Option<&str>,
    names: impl ExactSizeIterator<Item = Result<T>>,
) -> Result<NbtSummaryValue> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    if let Some(tag) = tag {
        let tag = bbb_protocol::normalize_resource_location(tag)
            .map_err(|error| invalid(error.to_string()))?;
        return Ok(NbtSummaryValue::String(format!("#{tag}").into()));
    }
    let mut values = Vec::with_capacity(names.len());
    for name in names {
        values.push(NbtSummaryValue::string(name?.as_ref()));
    }
    if values.len() == 1 {
        return Ok(values.remove(0));
    }
    Ok(NbtSummaryValue::List(values))
}

/// Cap on initial `Vec::with_capacity` pre-allocation for untrusted NBT
/// collections before element validation runs.
pub(super) const MAX_PREALLOC_CAPACITY: usize = 64;
/// Upper bound matching `bbb-protocol`'s `MAX_DATA_COMPONENT_LIST_ITEMS`.
pub(super) const MAX_PERSISTENT_LIST_ITEMS: usize = 4096;
/// Upper bound matching `bbb-protocol`'s `MAX_PARTIAL_DATA_COMPONENT_PREDICATES`
/// (`DataComponentPredicate.java:25`).
pub(super) const MAX_PARTIAL_PREDICATES: usize = 64;
/// Upper bound matching `bbb-protocol`'s `MAX_DATA_COMPONENT_PREDICATE_ENTRIES`.
pub(super) const MAX_EXACT_PREDICATE_ENTRIES: usize = 1024;

pub(super) fn project_compact_holder_set(
    c: ComponentReader,
    value: &NbtSummaryValue,
    holder_id: impl Fn(&bbb_protocol::JavaString) -> Result<i32>,
) -> Result<(Vec<i32>, Option<String>)> {
    if let Some(named) = nbt_string(value) {
        if let Some(tag) = named.units().strip_prefix(&[b'#' as u16]) {
            return Ok((Vec::new(), Some(normalize_resource_id_units(tag)?)));
        }
        return Ok((vec![holder_id(named)?], None));
    }
    let entries = c.list(value, "holder set")?;
    if entries.len() > MAX_PERSISTENT_LIST_ITEMS {
        return Err(c.error(format!(
            "holder set has {} entries, maximum is {MAX_PERSISTENT_LIST_ITEMS}",
            entries.len()
        )));
    }
    let mut registry_ids = Vec::with_capacity(entries.len().min(MAX_PREALLOC_CAPACITY));
    for entry in entries.iter() {
        registry_ids.push(holder_id(c.string(&entry, "holder set entry")?)?);
    }
    Ok((registry_ids, None))
}

/// `SuspiciousStewEffects.DEFAULT_DURATION`
pub(super) const PERSISTENT_SUSPICIOUS_STEW_DEFAULT_DURATION: i32 = 160;

/// `PotDecorations.CODEC` reads at most four item names
/// and `ordered` always writes that many
pub(super) const PERSISTENT_POT_DECORATION_SIDES: usize = 4;

pub(super) fn normalize_resource_id(resource_id: &str) -> Result<String> {
    bbb_protocol::normalize_resource_location(resource_id).map_err(|_| {
        super::error::ItemStackPersistentProjectionError::InvalidWireProjection {
            detail: format!("{resource_id} is not a resource location"),
        }
    })
}

pub(super) fn normalize_resource_id_units(units: &[u16]) -> Result<String> {
    let invalid =
        |detail| super::error::ItemStackPersistentProjectionError::InvalidWireProjection { detail };
    let value = String::from_utf16(units).map_err(|error| invalid(error.to_string()))?;
    bbb_protocol::normalize_resource_location(value).map_err(|error| invalid(error.to_string()))
}

pub(super) fn nbt_string(value: &NbtSummaryValue) -> Option<&bbb_protocol::JavaString> {
    match value {
        NbtSummaryValue::String(value) => Some(value),
        _ => None,
    }
}

pub(super) fn compound_fields(value: &NbtSummaryValue) -> Option<&[NbtSummaryEntry]> {
    match value {
        NbtSummaryValue::Compound(fields) => Some(fields),
        _ => None,
    }
}

pub(super) fn latest_field<'a>(
    fields: &'a [NbtSummaryEntry],
    name: &str,
) -> Option<&'a NbtSummaryValue> {
    fields
        .iter()
        .rev()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

/// Zero-allocation view over any `NbtOps.getList` `CollectionTag` (`List`,
/// `ByteArray`, `IntArray`, `LongArray`).
#[derive(Clone, Copy)]
pub(super) enum NbtListView<'a> {
    List(&'a [NbtSummaryValue]),
    ByteArray(&'a [i8]),
    IntArray(&'a [i32]),
    LongArray(&'a [i64]),
}

impl<'a> NbtListView<'a> {
    pub(super) const EMPTY: Self = Self::List(&[]);

    pub(super) fn len(self) -> usize {
        match self {
            Self::List(values) => values.len(),
            Self::ByteArray(values) => values.len(),
            Self::IntArray(values) => values.len(),
            Self::LongArray(values) => values.len(),
        }
    }

    pub(super) fn is_empty(self) -> bool {
        self.len() == 0
    }

    pub(super) fn iter(self) -> NbtListIter<'a> {
        NbtListIter {
            view: self,
            index: 0,
        }
    }

    pub(super) fn to_vec(self) -> Vec<NbtSummaryValue> {
        match self {
            Self::List(values) => values.to_vec(),
            Self::ByteArray(values) => values.iter().copied().map(NbtSummaryValue::Byte).collect(),
            Self::IntArray(values) => values.iter().copied().map(NbtSummaryValue::Int).collect(),
            Self::LongArray(values) => values.iter().copied().map(NbtSummaryValue::Long).collect(),
        }
    }
}

pub(super) struct NbtListIter<'a> {
    view: NbtListView<'a>,
    index: usize,
}

impl<'a> Iterator for NbtListIter<'a> {
    type Item = Cow<'a, NbtSummaryValue>;

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.index;
        if idx >= self.view.len() {
            return None;
        }
        self.index = idx + 1;
        Some(match self.view {
            NbtListView::List(values) => Cow::Borrowed(&values[idx]),
            NbtListView::ByteArray(values) => Cow::Owned(NbtSummaryValue::Byte(values[idx])),
            NbtListView::IntArray(values) => Cow::Owned(NbtSummaryValue::Int(values[idx])),
            NbtListView::LongArray(values) => Cow::Owned(NbtSummaryValue::Long(values[idx])),
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.view.len() - self.index;
        // Cap the lower bound so `Iterator::collect` never pre-allocates huge
        // buffers for untrusted primitive arrays before validating element 0.
        (remaining.min(MAX_PREALLOC_CAPACITY), Some(remaining))
    }
}

/// `NbtOps.getList` reads every `CollectionTag`, so a
/// list-valued field also reads the specialized array tags lazily without
/// boxing every element up front.
pub(super) fn list_entries(value: &NbtSummaryValue) -> Option<NbtListView<'_>> {
    Some(match value {
        NbtSummaryValue::List(values) => NbtListView::List(values),
        NbtSummaryValue::ByteArray(values) => NbtListView::ByteArray(values),
        NbtSummaryValue::IntArray(values) => NbtListView::IntArray(values),
        NbtSummaryValue::LongArray(values) => NbtListView::LongArray(values),
        _ => return None,
    })
}

/// The component a read is projecting, so a malformed field refuses the whole
/// component with its name instead of panicking.
#[derive(Clone, Copy)]
pub(super) struct ComponentReader {
    pub(super) type_id: i32,
    pub(super) resource_id: &'static str,
}

impl ComponentReader {
    pub(super) fn new(type_id: i32, resource_id: &'static str) -> Self {
        Self {
            type_id,
            resource_id,
        }
    }

    pub(super) fn error(
        self,
        detail: impl Into<String>,
    ) -> super::error::ItemStackPersistentProjectionError {
        invalid_persistent_component(self.type_id, self.resource_id, detail.into())
    }

    pub(super) fn compound<'a>(
        self,
        value: &'a NbtSummaryValue,
        what: &str,
    ) -> Result<&'a [NbtSummaryEntry]> {
        compound_fields(value).ok_or_else(|| self.error(format!("{what} is not a compound")))
    }

    pub(super) fn field<'a>(
        self,
        fields: &'a [NbtSummaryEntry],
        name: &str,
    ) -> Result<&'a NbtSummaryValue> {
        latest_field(fields, name)
            .ok_or_else(|| self.error(format!("missing required field {name}")))
    }

    pub(super) fn list<'a>(
        self,
        value: &'a NbtSummaryValue,
        what: &str,
    ) -> Result<NbtListView<'a>> {
        list_entries(value).ok_or_else(|| self.error(format!("{what} is not a list")))
    }

    pub(super) fn int<'a>(
        self,
        value: impl Into<crate::registry_codec::RegistryElementValueRef<'a>>,
        what: &str,
    ) -> Result<i32> {
        value
            .into()
            .codec_i32()
            .ok_or_else(|| self.error(format!("{what} is not numeric")))
    }

    pub(super) fn int_in<'a>(
        self,
        value: impl Into<crate::registry_codec::RegistryElementValueRef<'a>>,
        what: &str,
        min: i32,
        max: i32,
    ) -> Result<i32> {
        let value = self.int(value, what)?;
        if !(min..=max).contains(&value) {
            return Err(self.error(format!("{what} {value} is outside {min}..={max}")));
        }
        Ok(value)
    }

    pub(super) fn float(self, value: &NbtSummaryValue, what: &str) -> Result<f32> {
        bbb_protocol::nbt_summary::codec_float_value(value)
            .ok_or_else(|| self.error(format!("{what} is not numeric")))
    }

    pub(super) fn float_in(
        self,
        value: &NbtSummaryValue,
        what: &str,
        min: f32,
        max: f32,
    ) -> Result<f32> {
        checked_float(
            self.type_id,
            self.resource_id,
            self.float(value, what)?,
            min,
            max,
            what,
        )
    }

    /// `Codec.BOOL` compares the numeric tag's byte value with zero.
    pub(super) fn flag<'a>(
        self,
        value: impl Into<crate::registry_codec::RegistryElementValueRef<'a>>,
        what: &str,
    ) -> Result<bool> {
        value
            .into()
            .as_bool()
            .ok_or_else(|| self.error(format!("{what} is not numeric")))
    }

    pub(super) fn optional_flag(
        self,
        fields: &[NbtSummaryEntry],
        name: &str,
        default: bool,
    ) -> Result<bool> {
        latest_field(fields, name)
            .map(|value| self.flag(value, name))
            .transpose()
            .map(|value| value.unwrap_or(default))
    }

    pub(super) fn string<'a>(
        self,
        value: &'a NbtSummaryValue,
        what: &str,
    ) -> Result<&'a bbb_protocol::JavaString> {
        nbt_string(value).ok_or_else(|| self.error(format!("{what} is not a string")))
    }

    pub(super) fn java_string(
        self,
        value: &NbtSummaryValue,
        what: &str,
    ) -> Result<bbb_protocol::JavaString> {
        bbb_protocol::JavaString::from_nbt(value)
            .ok_or_else(|| self.error(format!("{what} is not a string")))
    }

    /// An `Identifier`, normalized to `namespace:path`.
    pub(super) fn resource(self, value: &NbtSummaryValue, what: &str) -> Result<String> {
        self.resource_units(self.string(value, what)?.units(), what)
    }

    pub(super) fn resource_units(self, units: &[u16], what: &str) -> Result<String> {
        normalize_resource_id_units(units)
            .map_err(|_| self.error(format!("{what} is not a resource location")))
    }

    /// `ComponentSerialization.CODEC`.
    pub(super) fn text(self, value: &NbtSummaryValue, what: &str) -> Result<()> {
        bbb_protocol::component::validate_component_nbt(value, what)
            .map_err(|error| self.error(error))
    }

    pub(super) fn text_with_registry(
        self,
        value: &NbtSummaryValue,
        what: &str,
        content_registry: &bbb_protocol::content_registry::ContentRegistry,
    ) -> Result<()> {
        bbb_protocol::component::validate_component_nbt_with_registry(value, what, content_registry)
            .map_err(|error| self.error(error))
    }

    pub(super) fn unit(self, value: &NbtSummaryValue) -> Result<()> {
        self.compound(value, "unit value").map(drop)
    }
}
