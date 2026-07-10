//! Vanilla 26.1 data-component catalog facade.

pub use crate::packets::data_components::catalog::{
    vanilla_data_component_descriptor, vanilla_data_component_descriptors, DataComponentDescriptor,
};

const VANILLA_DATA_COMPONENT_PREDICATE_RESOURCE_IDS: [&str; 15] = [
    "minecraft:damage",
    "minecraft:enchantments",
    "minecraft:stored_enchantments",
    "minecraft:potion_contents",
    "minecraft:custom_data",
    "minecraft:container",
    "minecraft:bundle_contents",
    "minecraft:firework_explosion",
    "minecraft:fireworks",
    "minecraft:writable_book_content",
    "minecraft:written_book_content",
    "minecraft:attribute_modifiers",
    "minecraft:trim",
    "minecraft:jukebox_playable",
    "minecraft:villager/variant",
];

pub fn vanilla_data_component_resource_id_for_type_id(type_id: i32) -> Option<&'static str> {
    vanilla_data_component_descriptor(type_id).map(|descriptor| descriptor.resource_id())
}

pub fn vanilla_data_component_type_is_persistent(type_id: i32) -> bool {
    vanilla_data_component_descriptor(type_id).is_some_and(|descriptor| descriptor.is_persistent())
}

pub fn vanilla_data_component_predicate_resource_id(type_id: i32) -> Option<&'static str> {
    VANILLA_DATA_COMPONENT_PREDICATE_RESOURCE_IDS
        .get(usize::try_from(type_id).ok()?)
        .copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compatibility_queries_delegate_to_catalog() {
        assert_eq!(
            vanilla_data_component_resource_id_for_type_id(0),
            Some("minecraft:custom_data")
        );
        assert_eq!(
            vanilla_data_component_resource_id_for_type_id(109),
            Some("minecraft:shulker/color")
        );
        assert_eq!(vanilla_data_component_resource_id_for_type_id(-1), None);
        assert_eq!(vanilla_data_component_resource_id_for_type_id(110), None);
        assert!(vanilla_data_component_type_is_persistent(0));
        assert!(!vanilla_data_component_type_is_persistent(20));
        assert!(!vanilla_data_component_type_is_persistent(41));
        assert!(!vanilla_data_component_type_is_persistent(48));
    }

    #[test]
    fn data_component_predicate_types_follow_vanilla_bootstrap_order() {
        assert_eq!(
            VANILLA_DATA_COMPONENT_PREDICATE_RESOURCE_IDS,
            [
                "minecraft:damage",
                "minecraft:enchantments",
                "minecraft:stored_enchantments",
                "minecraft:potion_contents",
                "minecraft:custom_data",
                "minecraft:container",
                "minecraft:bundle_contents",
                "minecraft:firework_explosion",
                "minecraft:fireworks",
                "minecraft:writable_book_content",
                "minecraft:written_book_content",
                "minecraft:attribute_modifiers",
                "minecraft:trim",
                "minecraft:jukebox_playable",
                "minecraft:villager/variant",
            ]
        );
        let unique = VANILLA_DATA_COMPONENT_PREDICATE_RESOURCE_IDS
            .into_iter()
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(unique.len(), 15);
        assert_eq!(vanilla_data_component_predicate_resource_id(-1), None);
        assert_eq!(vanilla_data_component_predicate_resource_id(15), None);
    }
}
