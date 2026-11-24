use bbb_protocol::{packets::ItemStackSummary, NbtSummaryEntry, NbtSummaryValue};
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::save::SaveResult;

use super::field_patches::{
    encode_player_inventory_slots, encode_stack, write_encoded_player_inventory,
    write_player_inventory,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Java26PlayerInventory {
    source: InventorySource,
}

#[derive(Debug, Clone, PartialEq)]
enum InventorySource {
    Legacy(Vec<ItemStackSummary>),
    Prepared {
        slots: Vec<RuntimeItemStack>,
        inventory: Vec<NbtSummaryValue>,
        equipment: Vec<NbtSummaryEntry>,
    },
}

impl From<Vec<ItemStackSummary>> for Java26PlayerInventory {
    fn from(slots: Vec<ItemStackSummary>) -> Self {
        Self {
            source: InventorySource::Legacy(slots),
        }
    }
}

impl Java26PlayerInventory {
    pub fn prepare(
        slots: &[RuntimeItemStack],
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> SaveResult<Self> {
        let (inventory, equipment) =
            encode_player_inventory_slots(slots, RuntimeItemStack::is_empty, |stack, path| {
                encode_stack(stack.summary(), resolver, || path)
            })?;
        Ok(Self {
            source: InventorySource::Prepared {
                slots: slots.to_vec(),
                inventory,
                equipment,
            },
        })
    }

    pub fn runtime_slots(&self) -> Vec<RuntimeItemStack> {
        match &self.source {
            InventorySource::Legacy(slots) => slots
                .iter()
                .cloned()
                .map(RuntimeItemStack::from_summary)
                .collect(),
            InventorySource::Prepared { slots, .. } => slots.clone(),
        }
    }

    pub fn prepared_fields(&self) -> Option<(&[NbtSummaryValue], &[NbtSummaryEntry])> {
        match &self.source {
            InventorySource::Legacy(_) => None,
            InventorySource::Prepared {
                inventory,
                equipment,
                ..
            } => Some((inventory, equipment)),
        }
    }

    pub(super) fn write(
        &self,
        entries: &mut Vec<NbtSummaryEntry>,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> SaveResult<()> {
        match &self.source {
            InventorySource::Legacy(slots) => write_player_inventory(entries, slots, resolver),
            InventorySource::Prepared {
                inventory,
                equipment,
                ..
            } => {
                write_encoded_player_inventory(entries, inventory.clone(), equipment.clone());
                Ok(())
            }
        }
    }
}
