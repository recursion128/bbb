use bbb_protocol::nbt_summary::entry;
use bbb_protocol::NbtSummaryValue;
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_types::item_stack::RuntimeItemStack;

/// `ItemStack.matches` (`ItemStack.java:612-618`) against
/// `Raid.getOminousBannerInstance` (`Raid.java:655-657`): the same item, the
/// same count and equal components, which the retained stack compares whole.
pub(crate) fn is_ominous_banner(
    stack: &RuntimeItemStack,
    items: &dyn PersistentItemStackHolderResolver,
) -> bool {
    if stack.count() != 1
        || stack.item_resource_id() != Some("minecraft:white_banner")
        || stack.summary().component_patch.banner_pattern_layers.len() != 8
    {
        return false;
    }
    stack == &ominous_banner_stack(items)
}

pub(crate) fn ominous_banner_stack(
    items: &dyn PersistentItemStackHolderResolver,
) -> RuntimeItemStack {
    // `Raid.java:632-656`.
    let layers = [
        ("minecraft:rhombus", "cyan"),
        ("minecraft:stripe_bottom", "light_gray"),
        ("minecraft:stripe_center", "gray"),
        ("minecraft:border", "light_gray"),
        ("minecraft:stripe_middle", "black"),
        ("minecraft:half_horizontal", "light_gray"),
        ("minecraft:circle", "light_gray"),
        ("minecraft:border", "black"),
    ]
    .into_iter()
    .map(|(pattern, color)| {
        NbtSummaryValue::Compound(vec![
            entry("pattern", NbtSummaryValue::string(pattern)),
            entry("color", NbtSummaryValue::string(color)),
        ])
    })
    .collect();
    RuntimeItemStack::read(
        &NbtSummaryValue::Compound(vec![
            entry("id", NbtSummaryValue::string("minecraft:white_banner")),
            entry("count", NbtSummaryValue::Int(1)),
            entry(
                "components",
                NbtSummaryValue::Compound(vec![
                    entry("minecraft:banner_patterns", NbtSummaryValue::List(layers)),
                    entry(
                        "minecraft:tooltip_display",
                        NbtSummaryValue::Compound(vec![entry(
                            "hidden_components",
                            NbtSummaryValue::List(vec![NbtSummaryValue::string(
                                "minecraft:banner_patterns".to_owned(),
                            )]),
                        )]),
                    ),
                    entry(
                        "minecraft:item_name",
                        NbtSummaryValue::Compound(vec![entry(
                            "translate",
                            NbtSummaryValue::string("block.minecraft.ominous_banner"),
                        )]),
                    ),
                    entry("minecraft:rarity", NbtSummaryValue::string("uncommon")),
                ]),
            ),
        ]),
        items,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::test_items::BuiltinItems;

    #[test]
    fn ominous_banner_keeps_its_patterns_through_a_save() {
        let stack = ominous_banner_stack(&BuiltinItems);
        assert_eq!(stack.item_resource_id(), Some("minecraft:white_banner"));
        assert_eq!(
            stack.summary().component_patch.banner_pattern_layers.len(),
            8
        );
        let saved = stack.write(&BuiltinItems).unwrap().unwrap();
        assert_eq!(RuntimeItemStack::read(&saved, &BuiltinItems), stack);
        assert!(is_ominous_banner(&stack, &BuiltinItems));
        assert!(!is_ominous_banner(&stack.with_count(2), &BuiltinItems));
    }
}
