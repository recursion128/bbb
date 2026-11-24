//! Shared state for registry codecs whose value owners recurse into one another.
//!
//! Component, Dialog, `DataComponentPatch`, and `ItemStackTemplate` form one
//! recursion family. They must consume one budget and retain dynamic registry
//! holders until the complete candidate generation can validate membership.

use crate::RegistryElementSource;
use bbb_protocol::{
    content_registry::ContentRegistry, nbt_summary::entry, JavaString, NbtSummaryEntry,
    NbtSummaryValue,
};

use super::{
    RegistryCodecError, RegistryCodecReference, RegistryElementValueRef, RegistryReferenceKind,
    RegistryResourceIdentifier,
};

pub(crate) use crate::PersistentNbtDecodeLimits as RegistryCodecDecodeLimits;

/// One decode context shared by every inline value in the recursive family.
///
/// Named holders never enter their target value. Inline owners receive this
/// exact mutable context, so moving between Component, Dialog, a component
/// patch, and an item template cannot reset the safety budget.
#[derive(Debug)]
pub(crate) struct RegistryCodecDecodeContext<'a> {
    owner_registry: &'a str,
    owner_codec: &'a str,
    source: RegistryElementSource,
    limits: RegistryCodecDecodeLimits,
    content_registry: &'a ContentRegistry,
    depth: usize,
    consumed_nodes: usize,
}

impl<'a> RegistryCodecDecodeContext<'a> {
    pub(crate) fn new(
        owner_registry: &'a str,
        owner_codec: &'a str,
        source: RegistryElementSource,
    ) -> Self {
        Self {
            owner_registry,
            owner_codec,
            source,
            limits: RegistryCodecDecodeLimits::DEFAULT,
            content_registry: ContentRegistry::vanilla().as_ref(),
            depth: 0,
            consumed_nodes: 0,
        }
    }

    pub(crate) fn with_limits(
        owner_registry: &'a str,
        owner_codec: &'a str,
        source: RegistryElementSource,
        limits: RegistryCodecDecodeLimits,
    ) -> Result<Self, RegistryCodecError> {
        if limits.max_depth == 0 || limits.max_nodes == 0 || limits.max_collection == 0 {
            return Err(RegistryCodecError::native_semantic(
                owner_registry,
                owner_codec,
                "<budget>",
                "registry codec limits must all be positive",
            )
            .aborting_speculation());
        }
        Ok(Self {
            owner_registry,
            owner_codec,
            source,
            limits,
            content_registry: ContentRegistry::vanilla().as_ref(),
            depth: 0,
            consumed_nodes: 0,
        })
    }

    pub(crate) fn with_content_registry(mut self, content_registry: &'a ContentRegistry) -> Self {
        self.content_registry = content_registry;
        self
    }

    pub(crate) fn content_registry(&self) -> &'a ContentRegistry {
        self.content_registry
    }

    pub(crate) fn source(&self) -> RegistryElementSource {
        self.source
    }

    pub(crate) fn owner_registry(&self) -> &str {
        self.owner_registry
    }

    pub(crate) fn owner_codec(&self) -> &str {
        self.owner_codec
    }

    pub(crate) fn charge_node(&mut self, field: &str) -> Result<(), RegistryCodecError> {
        self.charge_nodes(field, 1)
    }

    /// Charges scalar members represented compactly by a primitive NBT array.
    ///
    /// NbtOps exposes these members as an ordinary stream, so they count
    /// toward the same node limit as JSON/list children without increasing
    /// nesting depth.
    pub(crate) fn charge_nodes(
        &mut self,
        field: &str,
        count: usize,
    ) -> Result<(), RegistryCodecError> {
        self.consumed_nodes = self.consumed_nodes.checked_add(count).ok_or_else(|| {
            self.budget_error(
                field,
                format!("node count exceeds {}", self.limits.max_nodes),
            )
        })?;
        if self.consumed_nodes > self.limits.max_nodes {
            return Err(self.budget_error(
                field,
                format!("node count exceeds {}", self.limits.max_nodes),
            ));
        }
        Ok(())
    }

    pub(crate) fn decode_bool(
        &mut self,
        input: RegistryElementValueRef<'_>,
        field: &str,
    ) -> Result<bool, RegistryCodecError> {
        self.charge_node(field)?;
        input
            .as_bool()
            .ok_or_else(|| self.codec_error(field, "expected Codec.BOOL input"))
    }

    pub(crate) fn decode_f32(
        &mut self,
        input: RegistryElementValueRef<'_>,
        field: &str,
    ) -> Result<f32, RegistryCodecError> {
        self.charge_node(field)?;
        input
            .codec_f32()
            .ok_or_else(|| self.codec_error(field, "expected Codec.FLOAT input"))
    }

    pub(crate) fn decode_f64(
        &mut self,
        input: RegistryElementValueRef<'_>,
        field: &str,
    ) -> Result<f64, RegistryCodecError> {
        self.charge_node(field)?;
        input
            .codec_f64()
            .ok_or_else(|| self.codec_error(field, "expected Codec.DOUBLE input"))
    }

    pub(crate) fn decode_i32(
        &mut self,
        input: RegistryElementValueRef<'_>,
        field: &str,
    ) -> Result<i32, RegistryCodecError> {
        self.charge_node(field)?;
        input
            .codec_i32()
            .ok_or_else(|| self.codec_error(field, "expected Codec.INT input"))
    }

    pub(crate) fn decode_i32_range(
        &mut self,
        input: RegistryElementValueRef<'_>,
        field: &str,
        minimum: i32,
        maximum: i32,
    ) -> Result<i32, RegistryCodecError> {
        let value = self.decode_i32(input, field)?;
        if !(minimum..=maximum).contains(&value) {
            return Err(
                self.codec_error(field, format!("integer is outside {minimum}..={maximum}"))
            );
        }
        Ok(value)
    }

    pub(crate) fn charge_record(
        &mut self,
        input: RegistryElementValueRef<'_>,
        field: &str,
        expected: &str,
    ) -> Result<usize, RegistryCodecError> {
        self.charge_node(field)?;
        let len = input
            .map_len()
            .ok_or_else(|| self.codec_error(field, format!("{expected} is not a map")))?;
        self.check_collection(field, len)?;
        Ok(len)
    }

    pub(crate) fn check_collection(
        &self,
        field: &str,
        len: usize,
    ) -> Result<(), RegistryCodecError> {
        if len > self.limits.max_collection {
            return Err(self.budget_error(
                field,
                format!(
                    "collection has {len} entries, maximum is {}",
                    self.limits.max_collection
                ),
            ));
        }
        Ok(())
    }

    pub(crate) fn with_nested<T>(
        &mut self,
        field: &str,
        decode: impl FnOnce(&mut Self) -> Result<T, RegistryCodecError>,
    ) -> Result<T, RegistryCodecError> {
        let nested_depth = self.depth.checked_add(1).ok_or_else(|| {
            self.budget_error(
                field,
                format!("nesting exceeds depth {}", self.limits.max_depth),
            )
        })?;
        if nested_depth > self.limits.max_depth {
            return Err(self.budget_error(
                field,
                format!("nesting exceeds depth {}", self.limits.max_depth),
            ));
        }

        let previous_depth = self.depth;
        self.depth = nested_depth;
        let result = decode(self);
        self.depth = previous_depth;
        result
    }

    /// Rejected semantic alternatives must not double-charge the selected value;
    /// safety-budget and owner failures keep their charges and abort fallback.
    pub(crate) fn speculate<T>(
        &mut self,
        decode: impl FnOnce(&mut Self) -> Result<T, RegistryCodecError>,
    ) -> Result<T, RegistryCodecError> {
        let consumed_nodes = self.consumed_nodes;
        let result = decode(self);
        if matches!(&result, Err(error) if !error.abort_speculation) {
            self.consumed_nodes = consumed_nodes;
        }
        result
    }

    /// Only semantic branch mismatches may select another DFU branch; safety and
    /// owner failures must not change the selected value through fallback.
    pub(crate) fn speculate_codec_alternative<T>(
        &mut self,
        decode: impl FnOnce(&mut Self) -> Result<T, RegistryCodecError>,
    ) -> Result<Option<T>, RegistryCodecError> {
        match self.speculate(decode) {
            Ok(value) => Ok(Some(value)),
            Err(error) if error.abort_speculation => Err(error),
            Err(_) => Ok(None),
        }
    }

    pub(crate) fn budget_error(
        &self,
        field: &str,
        reason: impl Into<String>,
    ) -> RegistryCodecError {
        self.codec_error(field, reason).aborting_speculation()
    }

    pub(crate) fn internal_error(
        &self,
        field: &str,
        reason: impl Into<String>,
    ) -> RegistryCodecError {
        self.codec_error(field, reason).aborting_speculation()
    }

    pub(crate) fn coverage_error(
        &self,
        field: &str,
        reason: impl Into<String>,
    ) -> RegistryCodecError {
        self.codec_error(field, reason).aborting_speculation()
    }

    /// Builds an error attributed to the enclosing registry element codec.
    pub(crate) fn codec_error(&self, field: &str, reason: impl Into<String>) -> RegistryCodecError {
        RegistryCodecError::native_semantic(self.owner_registry, self.owner_codec, field, reason)
    }
}

pub(crate) fn required_field<'a>(
    input: RegistryElementValueRef<'a>,
    name: &str,
    field: &str,
    context: &RegistryCodecDecodeContext<'_>,
) -> Result<RegistryElementValueRef<'a>, RegistryCodecError> {
    input
        .field(name)
        .ok_or_else(|| context.codec_error(&format!("{field}.{name}"), "missing required field"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedRegistryCodecReference {
    pub target_registry: &'static str,
    pub kind: RegistryReferenceKind,
    pub resource_id: RegistryResourceIdentifier,
}

impl OwnedRegistryCodecReference {
    pub(crate) fn element(
        target_registry: &'static str,
        resource_id: RegistryResourceIdentifier,
    ) -> Self {
        Self {
            target_registry,
            kind: RegistryReferenceKind::Element,
            resource_id,
        }
    }

    pub(crate) fn tag(
        target_registry: &'static str,
        resource_id: RegistryResourceIdentifier,
    ) -> Self {
        Self {
            target_registry,
            kind: RegistryReferenceKind::Tag,
            resource_id,
        }
    }

    pub(crate) fn borrowed(&self) -> RegistryCodecReference<'_> {
        RegistryCodecReference {
            target_registry: self.target_registry,
            kind: self.kind,
            resource_id: &self.resource_id,
        }
    }
}

/// Canonical NbtOps output plus unresolved dynamic-registry dependencies.
///
/// The NBT is not publication-authorized by itself. The enclosing registry
/// element must expose [`Self::references`] and require the shared frozen
/// lookup before cloning [`Self::network_nbt_unchecked`] into a packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalRegistryCodecValue {
    pub network_nbt: NbtSummaryValue,
    pub references: Vec<OwnedRegistryCodecReference>,
}

impl CanonicalRegistryCodecValue {
    pub(crate) fn new(network_nbt: NbtSummaryValue) -> Self {
        Self::from_parts(network_nbt, Vec::new())
    }

    pub(crate) fn from_parts(
        network_nbt: NbtSummaryValue,
        references: Vec<OwnedRegistryCodecReference>,
    ) -> Self {
        Self {
            network_nbt,
            references,
        }
    }

    pub(crate) fn push_reference(&mut self, reference: OwnedRegistryCodecReference) {
        self.references.push(reference);
    }

    pub(crate) fn references(&self) -> impl Iterator<Item = RegistryCodecReference<'_>> {
        self.references
            .iter()
            .map(OwnedRegistryCodecReference::borrowed)
    }

    pub fn network_nbt_unchecked(&self) -> &NbtSummaryValue {
        &self.network_nbt
    }

    pub(crate) fn into_parts(self) -> (NbtSummaryValue, Vec<OwnedRegistryCodecReference>) {
        (self.network_nbt, self.references)
    }
}

pub(crate) fn decode_list(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    decode: impl FnMut(
        RegistryElementValueRef<'_>,
        &str,
        &mut RegistryCodecDecodeContext<'_>,
    ) -> Result<CanonicalRegistryCodecValue, RegistryCodecError>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    let items = input
        .list_elements()
        .ok_or_else(|| context.codec_error(field, "expected a list"))?;
    context.check_collection(field, items.len())?;
    decode_list_items(items, field, context, decode)
}

pub(crate) fn decode_compact_list(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    mut decode: impl FnMut(
        RegistryElementValueRef<'_>,
        &str,
        &mut RegistryCodecDecodeContext<'_>,
    ) -> Result<CanonicalRegistryCodecValue, RegistryCodecError>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    if let Some(mut items) = input.list_elements() {
        context.charge_node(field)?;
        context.check_collection(field, items.len())?;
        if items.len() == 1 {
            let item_field = format!("{field}[0]");
            let item = items.next().expect("one list element");
            return context.with_nested(&item_field, |context| decode(item, &item_field, context));
        }
        return decode_list_items(items, field, context, decode);
    }
    decode(input, field, context)
}

fn decode_list_items<'a>(
    items: impl ExactSizeIterator<Item = RegistryElementValueRef<'a>>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    mut decode: impl FnMut(
        RegistryElementValueRef<'_>,
        &str,
        &mut RegistryCodecDecodeContext<'_>,
    ) -> Result<CanonicalRegistryCodecValue, RegistryCodecError>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let mut values = Vec::with_capacity(items.len());
    let mut references = Vec::new();
    for (index, item) in items.enumerate() {
        let item_field = format!("{field}[{index}]");
        let decoded =
            context.with_nested(&item_field, |context| decode(item, &item_field, context))?;
        let (value, nested_references) = decoded.into_parts();
        values.push(value);
        references.extend(nested_references);
    }
    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::List(values),
        references,
    ))
}

#[derive(Debug, Default)]
pub(crate) struct RegistryCodecRecord {
    pub(crate) fields: Vec<NbtSummaryEntry>,
    pub(crate) references: Vec<OwnedRegistryCodecReference>,
}

impl RegistryCodecRecord {
    pub(crate) fn push(&mut self, name: impl Into<JavaString>, value: CanonicalRegistryCodecValue) {
        let (value, references) = value.into_parts();
        self.push_value(name, value);
        self.references.extend(references);
    }

    pub(crate) fn push_value(&mut self, name: impl Into<JavaString>, value: NbtSummaryValue) {
        self.fields.push(entry(name, value));
    }

    pub(crate) fn append(&mut self, mut other: Self) {
        self.fields.append(&mut other.fields);
        self.references.append(&mut other.references);
    }

    pub(crate) fn finish(self) -> CanonicalRegistryCodecValue {
        CanonicalRegistryCodecValue::from_parts(
            NbtSummaryValue::Compound(self.fields),
            self.references,
        )
    }

    pub(crate) fn finish_typed(
        mut self,
        type_id: impl Into<JavaString>,
    ) -> CanonicalRegistryCodecValue {
        self.push_value("type", NbtSummaryValue::String(type_id.into()));
        self.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use bbb_protocol::NbtSummaryEntry;

    #[test]
    fn canonical_value_keeps_network_nbt_and_borrowed_dependencies_together() {
        let mut value =
            CanonicalRegistryCodecValue::new(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "title".into(),
                value: NbtSummaryValue::String("hello".to_string().into()),
            }]));
        value.push_reference(OwnedRegistryCodecReference::element(
            "minecraft:dialog",
            RegistryResourceIdentifier::parse("example:next").unwrap(),
        ));
        value.push_reference(OwnedRegistryCodecReference::tag(
            "minecraft:item",
            RegistryResourceIdentifier::parse("example:tools").unwrap(),
        ));

        assert!(matches!(
            value.network_nbt_unchecked(),
            NbtSummaryValue::Compound(entries) if entries.len() == 1
        ));
        let references = value.references().collect::<Vec<_>>();
        assert_eq!(references.len(), 2);
        assert_eq!(references[0].target_registry, "minecraft:dialog");
        assert_eq!(references[0].kind, RegistryReferenceKind::Element);
        assert_eq!(references[0].resource_id.as_str(), "example:next");
        assert_eq!(references[1].target_registry, "minecraft:item");
        assert_eq!(references[1].kind, RegistryReferenceKind::Tag);
        assert_eq!(references[1].resource_id.as_str(), "example:tools");

        let (network_nbt, references) = value.into_parts();
        assert!(matches!(network_nbt, NbtSummaryValue::Compound(_)));
        assert_eq!(references.len(), 2);
    }
}
