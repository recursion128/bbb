//! Block-entity item input and output through the level's registry publication.

use bbb_protocol::NbtSummaryValue;
use bbb_registry_codec::{PersistentItemStackHolderResolver, UnavailableHolderResolver};
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result;

#[derive(Clone, Copy)]
pub struct BlockEntityItemProjection<'a> {
    resolver: Option<&'a dyn PersistentItemStackHolderResolver>,
}

impl<'a> BlockEntityItemProjection<'a> {
    pub const fn without_resolver() -> Self {
        Self { resolver: None }
    }
    pub const fn with_resolver(resolver: &'a dyn PersistentItemStackHolderResolver) -> Self {
        Self {
            resolver: Some(resolver),
        }
    }
    pub(super) const fn has_resolver(self) -> bool {
        self.resolver.is_some()
    }
    pub(super) fn resolver(self) -> &'a dyn PersistentItemStackHolderResolver {
        self.resolver.unwrap_or(&UnavailableHolderResolver)
    }
}

pub struct BlockEntityItemLoad<'a> {
    projection: BlockEntityItemProjection<'a>,
}
impl<'a> BlockEntityItemLoad<'a> {
    pub(super) fn resolver(&self) -> &'a dyn PersistentItemStackHolderResolver {
        self.projection.resolver()
    }
    pub const fn new(projection: BlockEntityItemProjection<'a>) -> Self {
        Self { projection }
    }
    /// An unreadable stack loads as absent.
    pub fn read(&mut self, value: &NbtSummaryValue) -> Result<Option<RuntimeItemStack>> {
        Ok(RuntimeItemStack::try_read(value, self.projection.resolver()).ok())
    }
    // ItemStackWithSlot.java:9-18: Slot defaults to zero.
    pub(super) fn read_with_slot(
        &mut self,
        value: &NbtSummaryValue,
    ) -> Result<Option<(u8, RuntimeItemStack)>> {
        let slot = match value {
            NbtSummaryValue::Compound(entries) => entries
                .iter()
                .rev()
                .find(|entry| entry.name == "Slot")
                .and_then(|entry| bbb_protocol::nbt_summary::codec_int_value(&entry.value))
                .unwrap_or(0) as u8,
            _ => 0,
        };
        Ok(self.read(value)?.map(|stack| (slot, stack)))
    }
}

pub struct BlockEntityItemSave<'a> {
    projection: BlockEntityItemProjection<'a>,
}
impl<'a> BlockEntityItemSave<'a> {
    pub(super) fn resolver(&self) -> &'a dyn PersistentItemStackHolderResolver {
        self.projection.resolver()
    }
    pub const fn new(projection: BlockEntityItemProjection<'a>) -> Self {
        Self { projection }
    }
    // TagValueOutput.java:225-234: a stack that cannot be encoded is omitted.
    pub fn write(&mut self, stack: &RuntimeItemStack) -> Result<Option<NbtSummaryValue>> {
        Ok(stack.write(self.projection.resolver()).ok().flatten())
    }
}
