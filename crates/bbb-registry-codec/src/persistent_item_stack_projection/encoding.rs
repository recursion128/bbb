//! Static output operations shared by persistent component schemas.

use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::nbt_summary::entry;
use bbb_protocol::packets::SoundEventSummary;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::error::{invalid_persistent_component, ItemStackPersistentProjectionError, Result};
use super::holder::persistent_sound_event_nbt;
use super::nbt_support::persistent_float_nbt;

pub enum ComponentScalar<'a> {
    Int(i32),
    Byte(i8),
    Boolean(bool),
    Float(u32),
    String(&'a str),
}

pub trait ComponentEncoder {
    type Value;
    type Fields;
    type Error;

    fn fields(&self, capacity: usize) -> Self::Fields;
    fn field(&self, fields: &mut Self::Fields, name: &'static str, value: Self::Value);
    fn record(&self, fields: Self::Fields) -> Self::Value;
    fn scalar(&self, fields: &mut Self::Fields, name: &'static str, value: ComponentScalar<'_>);
    fn list(
        &self,
        values: impl Iterator<Item = std::result::Result<Self::Value, Self::Error>>,
    ) -> std::result::Result<Self::Value, Self::Error>;
    fn check(&self, valid: bool, detail: &'static str) -> std::result::Result<(), Self::Error>;
    fn sound(&self, sound: &SoundEventSummary) -> std::result::Result<Self::Value, Self::Error>;
}

pub(super) struct ComponentNbtEncoder<'a, H> {
    pub type_id: i32,
    pub resource_id: &'static str,
    pub holder_resource_id: H,
    pub content_registry: &'a ContentRegistry,
}

impl<'a, H> ComponentNbtEncoder<'a, H> {
    pub(super) fn new(
        type_id: i32,
        resource_id: &'static str,
        holder_resource_id: H,
        content_registry: &'a ContentRegistry,
    ) -> Self {
        Self {
            type_id,
            resource_id,
            holder_resource_id,
            content_registry,
        }
    }
}

impl<H> ComponentEncoder for ComponentNbtEncoder<'_, H> {
    type Value = NbtSummaryValue;
    type Fields = Vec<NbtSummaryEntry>;
    type Error = ItemStackPersistentProjectionError;

    fn fields(&self, capacity: usize) -> Self::Fields {
        Vec::with_capacity(capacity)
    }

    fn field(&self, fields: &mut Self::Fields, name: &'static str, value: Self::Value) {
        fields.push(entry(name, value));
    }

    fn record(&self, fields: Self::Fields) -> Self::Value {
        NbtSummaryValue::Compound(fields)
    }

    #[inline]
    fn scalar(&self, fields: &mut Self::Fields, name: &'static str, value: ComponentScalar<'_>) {
        self.field(
            fields,
            name,
            match value {
                ComponentScalar::Int(value) => NbtSummaryValue::Int(value),
                ComponentScalar::Byte(value) => NbtSummaryValue::Byte(value),
                ComponentScalar::Boolean(value) => NbtSummaryValue::Byte(i8::from(value)),
                ComponentScalar::Float(bits) => persistent_float_nbt(bits),
                ComponentScalar::String(value) => NbtSummaryValue::String(value.into()),
            },
        );
    }

    fn list(&self, values: impl Iterator<Item = Result<Self::Value>>) -> Result<Self::Value> {
        let mut list = Vec::with_capacity(values.size_hint().0);
        for value in values {
            list.push(value?);
        }
        Ok(NbtSummaryValue::List(list))
    }

    fn check(&self, valid: bool, detail: &'static str) -> Result<()> {
        if valid {
            Ok(())
        } else {
            Err(invalid_persistent_component(
                self.type_id,
                self.resource_id,
                detail.to_string(),
            ))
        }
    }

    fn sound(&self, sound: &SoundEventSummary) -> Result<Self::Value> {
        persistent_sound_event_nbt(self.type_id, self.resource_id, sound, self.content_registry)
    }
}
