//! `BlockEntity.collectComponents` (`BlockEntity.java:309-314`): retained
//! components, then the type's implicit components.

use bbb_protocol::data_component_types::{
    vanilla_data_component_type, vanilla_data_component_type_for_resource_id,
    VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID, VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID,
    VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID,
};
use bbb_protocol::{normalize_resource_location, NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::item_stack::RuntimeItemStack;

use super::item_projection::BlockEntityItemProjection;

#[derive(Debug, Clone, PartialEq)]
pub struct CollectedComponent {
    pub id: String,
    pub value: Result<NbtSummaryValue, String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CollectedComponents {
    entries: Vec<CollectedComponent>,
}

impl CollectedComponents {
    #[must_use]
    pub fn entries(&self) -> &[CollectedComponent] {
        &self.entries
    }

    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Result<NbtSummaryValue, String>> {
        self.entries
            .iter()
            .find(|component| component.id == id)
            .map(|component| &component.value)
    }
}

pub(crate) struct ComponentMapBuilder<'a> {
    entries: Vec<CollectedComponent>,
    projection: BlockEntityItemProjection<'a>,
}

impl<'a> ComponentMapBuilder<'a> {
    pub(super) fn new(
        retained: &[NbtSummaryEntry],
        projection: BlockEntityItemProjection<'a>,
    ) -> Self {
        let mut builder = Self {
            entries: Vec::with_capacity(retained.len()),
            projection,
        };
        for component in retained {
            let Ok(name) = component.name.try_to_string() else {
                continue;
            };
            let id = normalize_resource_location(name.clone()).unwrap_or(name);
            builder.set_result(&id, Some(Ok(component.value.clone())));
        }
        builder
    }

    // DataComponentMap.java:128-139: null removes the type.
    pub(crate) fn set(&mut self, id: &str, value: Option<NbtSummaryValue>) {
        self.set_result(id, value.map(Ok));
    }

    fn set_result(&mut self, id: &str, value: Option<Result<NbtSummaryValue, String>>) {
        match (
            self.entries.iter().position(|component| component.id == id),
            value,
        ) {
            (Some(index), Some(value)) => self.entries[index].value = value,
            (Some(index), None) => {
                self.entries.remove(index);
            }
            (None, Some(value)) => self.entries.push(CollectedComponent {
                id: id.to_string(),
                value,
            }),
            (None, None) => {}
        }
    }

    pub(crate) fn set_container(&mut self, items: &[RuntimeItemStack]) {
        let resolver = self.projection.resolver();
        let patch = bbb_server_types::item_stack::container_patch(items);
        let value = bbb_registry_codec::data_component_patch_to_persistent_nbt_with_resolver(
            &patch, resolver,
        )
        .map_err(|error| error.to_string())
        .and_then(|value| match value {
            Some(NbtSummaryValue::Compound(mut entries)) if entries.len() == 1 => {
                Ok(entries.remove(0).value)
            }
            _ => Err("container component has no persistent value".to_string()),
        });
        self.set_result("minecraft:container", Some(value));
    }

    pub(super) fn build(self) -> CollectedComponents {
        CollectedComponents {
            entries: self.entries,
        }
    }
}

/// The placed stack's components as `BlockEntity.applyComponentsFromItemStack`
/// reads them (`BlockEntity.java:287-300`).
pub(super) struct ComponentMapReader<'a> {
    source: &'a RuntimeItemStack,
    projection: BlockEntityItemProjection<'a>,
    consumed: Vec<i32>,
}

impl<'a> ComponentMapReader<'a> {
    pub(super) fn new(
        source: &'a RuntimeItemStack,
        projection: BlockEntityItemProjection<'a>,
    ) -> Self {
        Self {
            source,
            projection,
            consumed: vec![
                VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
                VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID,
            ],
        }
    }

    pub(super) fn read(&mut self, id: &str) -> bbb_server_types::Result<Option<NbtSummaryValue>> {
        let type_id = vanilla_data_component_type_for_resource_id(id)
            .expect("registered implicit component")
            .type_id;
        self.consumed.push(type_id);
        Ok(self
            .source
            .component_nbt(type_id, self.projection.resolver())
            .ok()
            .flatten())
    }

    fn contents(&mut self) -> Vec<RuntimeItemStack> {
        self.consumed.push(VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID);
        self.source.container_contents().unwrap_or_default()
    }

    pub(super) fn copy_container_into(
        &mut self,
        items: &mut [RuntimeItemStack],
    ) -> bbb_server_types::Result<()> {
        let mut contents = self.contents().into_iter();
        for item in items {
            *item = contents.next().unwrap_or_default();
        }
        Ok(())
    }

    pub(super) fn copy_one(&mut self) -> bbb_server_types::Result<RuntimeItemStack> {
        Ok(self.contents().into_iter().next().unwrap_or_default())
    }

    // BlockEntity.java:298-299: the patch's added components minus the consumed ones.
    pub(super) fn retained(self) -> Vec<NbtSummaryEntry> {
        let resolver = self.projection.resolver();
        let patch = &self.source.summary().component_patch;
        patch
            .added_type_ids
            .iter()
            .filter(|type_id| !self.consumed.contains(type_id))
            .filter_map(|&type_id| {
                let component = vanilla_data_component_type(type_id)?;
                if !component.persistent {
                    return None;
                }
                let value = self.source.component_nbt(type_id, resolver).ok()??;
                Some(NbtSummaryEntry {
                    name: component.resource_id.into(),
                    value,
                })
            })
            .collect()
    }
}
