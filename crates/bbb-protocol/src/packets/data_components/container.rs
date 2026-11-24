use serde::{ser::SerializeSeq, Serialize, Serializer};

use super::{ItemContainerSlotSummary, ItemStackTemplateSummary};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContainerContentsSummary {
    pub slots: Vec<ItemContainerSlotSummary>,
    /// Exact optional-slot stream span, including trailing empty slots.
    /// None means that the component is absent; Some(0) is an empty component.
    pub slot_count: Option<usize>,
}

impl ContainerContentsSummary {
    pub fn items(&self) -> impl ExactSizeIterator<Item = &ItemStackTemplateSummary> {
        self.slots.iter().map(|slot| &slot.item)
    }

    pub(super) fn validate(&self) -> Result<(), &'static str> {
        if self
            .slots
            .iter()
            .any(|slot| self.slot_count.is_none_or(|count| slot.slot >= count))
            || self
                .slots
                .windows(2)
                .any(|pair| pair[0].slot >= pair[1].slot)
        {
            return Err("container slots disagree with span or order");
        }
        Ok(())
    }
}

pub(super) struct Items<'a>(pub(super) &'a [ItemContainerSlotSummary]);

impl Serialize for Items<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for slot in self.0 {
            seq.serialize_element(&slot.item)?;
        }
        seq.end()
    }
}
