pub(super) use bbb_protocol::attribute_types::AttributeSentiment as VanillaAttributeSentiment;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct VanillaAttribute {
    pub key: &'static str,
    pub description_key: &'static str,
    pub sentiment: VanillaAttributeSentiment,
}

const fn attribute(
    key: &'static str,
    description_key: &'static str,
    sentiment: VanillaAttributeSentiment,
) -> VanillaAttribute {
    VanillaAttribute {
        key,
        description_key,
        sentiment,
    }
}

fn project_attribute(
    entry: &bbb_protocol::attribute_types::AttributeTypeCatalogEntry,
) -> VanillaAttribute {
    attribute(entry.resource_id, entry.description_key, entry.sentiment)
}

pub(super) fn vanilla_attribute(attribute_id: i32) -> Option<VanillaAttribute> {
    bbb_protocol::attribute_types::vanilla_attribute_type(attribute_id).map(project_attribute)
}

pub(super) fn vanilla_attribute_for_key(key: &str) -> Option<VanillaAttribute> {
    bbb_protocol::attribute_types::vanilla_attribute_type_for_resource_id(key)
        .map(project_attribute)
}

pub(super) fn vanilla_attribute_id_for_key(key: &str) -> Option<i32> {
    Some(bbb_protocol::attribute_types::vanilla_attribute_type_for_resource_id(key)?.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vanilla_attributes_follow_26_1_bootstrap_order() {
        assert_eq!(
            bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_TYPES.len(),
            35
        );
        assert_eq!(
            vanilla_attribute(2),
            Some(attribute(
                "minecraft:attack_damage",
                "attribute.name.attack_damage",
                VanillaAttributeSentiment::Positive,
            ))
        );
        assert_eq!(
            vanilla_attribute(16),
            Some(attribute(
                "minecraft:knockback_resistance",
                "attribute.name.knockback_resistance",
                VanillaAttributeSentiment::Positive,
            ))
        );
        assert_eq!(
            vanilla_attribute(25),
            Some(attribute(
                "minecraft:scale",
                "attribute.name.scale",
                VanillaAttributeSentiment::Neutral,
            ))
        );
        assert_eq!(
            vanilla_attribute_for_key("minecraft:attack_damage"),
            vanilla_attribute(2)
        );
        assert_eq!(
            vanilla_attribute_id_for_key("minecraft:attack_speed"),
            Some(4)
        );
        assert_eq!(
            vanilla_attribute_id_for_key("minecraft:generic.attack_speed"),
            None
        );
        assert_eq!(
            vanilla_attribute(34),
            Some(attribute(
                "minecraft:waypoint_receive_range",
                "attribute.name.waypoint_receive_range",
                VanillaAttributeSentiment::Neutral,
            ))
        );
        assert_eq!(vanilla_attribute(35), None);
        assert_eq!(vanilla_attribute(-1), None);
    }
}
