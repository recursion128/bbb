//! `ItemPredicate.test` (`ItemPredicate.java:26-32`) over a live stack.

use bbb_protocol::packets::PersistentItemPredicateSummary;
use bbb_protocol::PersistentRegistryTagMembers;
use bbb_registry_codec::PersistentItemStackHolderResolver;

use super::RuntimeItemStack;

impl RuntimeItemStack {
    // DataComponentExactPredicate.java:77-85.
    pub fn matches_component(
        &self,
        type_id: i32,
        expected: &bbb_protocol::NbtSummaryValue,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> bool {
        let Ok(expected) = bbb_registry_codec::read_item_component(type_id, expected, resolver)
        else {
            return false;
        };
        self.effective_component(type_id, resolver)
            .ok()
            .flatten()
            .is_some_and(|actual| {
                bbb_protocol::packets::data_components::fields::component_values_equal(
                    type_id, &expected, &actual,
                )
                .unwrap_or(false)
            })
    }
}

/// Partial component predicates are not evaluated; a predicate carrying one
/// never matches.
pub fn item_predicate_matches(
    predicate: &PersistentItemPredicateSummary,
    stack: &RuntimeItemStack,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> bool {
    if let Some(items) = &predicate.items {
        // ItemStack.is(HolderSet) reads air for an empty stack (`ItemStack.java:350-352`).
        let item_id = stack
            .item_id()
            .unwrap_or(bbb_protocol::item_types::VANILLA_ITEM_AIR_ID);
        let in_set = match &items.tag {
            Some(tag) => match resolver.tag_members("minecraft:item", tag) {
                PersistentRegistryTagMembers::Present(members) => {
                    let resource_id = bbb_protocol::item_types::vanilla_item_type(item_id)
                        .map(|item| item.resource_id)
                        .or_else(|| {
                            resolver
                                .content_registry()
                                .item_by_id(item_id)
                                .map(|item| item.resource_id.as_str())
                        });
                    resource_id.is_some_and(|id| members.iter().any(|member| member == id))
                }
                _ => false,
            },
            None => items.registry_ids.contains(&item_id),
        };
        if !in_set {
            return false;
        }
    }
    let count = stack.count();
    if predicate.count_min.is_some_and(|min| count < min)
        || predicate.count_max.is_some_and(|max| count > max)
    {
        return false;
    }
    if !predicate.partial_components.is_empty() {
        return false;
    }
    predicate.exact_components.iter().all(|expected| {
        stack.matches_component(expected.component_type_id, &expected.value, resolver)
    })
}
