use bbb_protocol::{
    nbt_summary::entry, packets::ItemStackSummary, NbtSummaryEntry, NbtSummaryValue,
};
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::save::SaveResult;

use super::field_patches::{encode_stack, write_player_ender_items};

#[derive(Debug, Clone, PartialEq)]
pub struct Java26PlayerEnderItems {
    source: EnderItemsSource,
}

#[derive(Debug, Clone, PartialEq)]
enum EnderItemsSource {
    Legacy(Vec<ItemStackSummary>),
    Prepared {
        slots: Vec<RuntimeItemStack>,
        output: Vec<(usize, NbtSummaryValue)>,
    },
}

impl From<Vec<ItemStackSummary>> for Java26PlayerEnderItems {
    fn from(slots: Vec<ItemStackSummary>) -> Self {
        Self {
            source: EnderItemsSource::Legacy(slots),
        }
    }
}

impl Java26PlayerEnderItems {
    pub fn prepare(
        slots: &[RuntimeItemStack],
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> SaveResult<Self> {
        let mut output = Vec::new();
        // PlayerEnderChestContainer.java:40-47: ascending slots, excluding empty stacks.
        for (index, stack) in slots.iter().enumerate() {
            if stack.is_empty() {
                continue;
            }
            let mut fields = encode_stack(stack.summary(), resolver, || {
                format!("player.EnderItems[{index}]")
            })?;
            fields.insert(0, entry("Slot", NbtSummaryValue::Byte(index as i8)));
            output.push((index, NbtSummaryValue::Compound(fields)));
        }
        Ok(Self {
            source: EnderItemsSource::Prepared {
                slots: slots.to_vec(),
                output,
            },
        })
    }

    pub fn runtime_slots(&self) -> Vec<RuntimeItemStack> {
        match &self.source {
            EnderItemsSource::Legacy(slots) => slots
                .iter()
                .cloned()
                .map(RuntimeItemStack::from_summary)
                .collect(),
            EnderItemsSource::Prepared { slots, .. } => slots.clone(),
        }
    }

    pub fn prepared_slots(&self) -> Option<impl Iterator<Item = (usize, &NbtSummaryValue)>> {
        match &self.source {
            EnderItemsSource::Legacy(_) => None,
            EnderItemsSource::Prepared { output, .. } => {
                Some(output.iter().map(|(slot, raw)| (*slot, raw)))
            }
        }
    }

    pub(super) fn write(
        &self,
        entries: &mut Vec<NbtSummaryEntry>,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> SaveResult<()> {
        match &self.source {
            EnderItemsSource::Legacy(slots) => write_player_ender_items(entries, slots, resolver),
            EnderItemsSource::Prepared { output, .. } => {
                let value =
                    NbtSummaryValue::List(output.iter().map(|(_, raw)| raw.clone()).collect());
                match entries.iter_mut().find(|field| field.name == "EnderItems") {
                    Some(field) => field.value = value,
                    None => entries.push(entry("EnderItems", value)),
                }
                Ok(())
            }
        }
    }
}
