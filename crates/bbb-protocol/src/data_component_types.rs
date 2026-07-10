//! Vanilla 26.1 data-component catalog facade.

pub use crate::packets::data_components::catalog::{
    vanilla_data_component_descriptor, vanilla_data_component_descriptors, DataComponentDescriptor,
};

pub fn vanilla_data_component_resource_id_for_type_id(type_id: i32) -> Option<&'static str> {
    vanilla_data_component_descriptor(type_id).map(|descriptor| descriptor.resource_id())
}

pub fn vanilla_data_component_type_is_persistent(type_id: i32) -> bool {
    vanilla_data_component_descriptor(type_id).is_some_and(|descriptor| descriptor.is_persistent())
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
}
