use bbb_protocol::{packets::inventory::ItemStackSummary, NbtSummaryEntry, NbtSummaryValue};

use crate::WorldStore;

use super::decode_command_item_stack_template_summary;

impl WorldStore {
    /// Projects the exact persistent `ItemStackTemplate` retained by a
    /// Component `show_item` hover event through the same registry-aware codec
    /// used by dialog/command item templates.
    ///
    /// Unsupported or stale registry/component values return `None`; native
    /// then keeps the bounded id/count fallback rather than inventing an
    /// incomplete stack.
    pub fn component_hover_item_stack(
        &self,
        resource_id: &str,
        count: i32,
        components: Option<&NbtSummaryValue>,
    ) -> Option<ItemStackSummary> {
        let mut entries = vec![
            NbtSummaryEntry {
                name: "id".to_string(),
                value: NbtSummaryValue::String(resource_id.to_string()),
            },
            NbtSummaryEntry {
                name: "count".to_string(),
                value: NbtSummaryValue::Int(count),
            },
        ];
        if let Some(components) = components {
            entries.push(NbtSummaryEntry {
                name: "components".to_string(),
                value: components.clone(),
            });
        }
        decode_command_item_stack_template_summary(
            &NbtSummaryValue::Compound(entries),
            self.frozen_registry_access(),
        )
        .ok()
    }
}

#[cfg(test)]
mod tests {
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

    use crate::WorldStore;

    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.to_string(),
            value,
        }
    }

    #[test]
    fn component_hover_item_stack_uses_the_shared_persistent_template_codec() {
        let world = WorldStore::new();
        let components = NbtSummaryValue::Compound(vec![entry(
            "minecraft:custom_name",
            NbtSummaryValue::Compound(vec![entry(
                "text",
                NbtSummaryValue::String("Hover Stone".to_string()),
            )]),
        )]);
        let stack = world
            .component_hover_item_stack("minecraft:stone", 3, Some(&components))
            .expect("valid built-in hover item");
        assert_eq!(stack.count, 3);
        assert_eq!(
            stack.component_patch.custom_name.as_deref(),
            Some("Hover Stone")
        );
        assert_eq!(
            stack.component_patch.custom_name_nbt,
            Some(NbtSummaryValue::Compound(vec![entry(
                "text",
                NbtSummaryValue::String("Hover Stone".to_string()),
            )]))
        );
    }

    #[test]
    fn component_hover_item_stack_rejects_invalid_count_and_component() {
        let world = WorldStore::new();
        assert!(world
            .component_hover_item_stack("minecraft:stone", 0, None)
            .is_none());
        assert!(world
            .component_hover_item_stack(
                "minecraft:stone",
                1,
                Some(&NbtSummaryValue::Compound(vec![entry(
                    "example:unknown",
                    NbtSummaryValue::Compound(Vec::new()),
                )])),
            )
            .is_none());
    }
}
