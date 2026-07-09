#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VanillaAttributeSentiment {
    Positive,
    Neutral,
    Negative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct VanillaAttribute {
    pub key: &'static str,
    pub description_key: &'static str,
    pub sentiment: VanillaAttributeSentiment,
}

const VANILLA_ATTRIBUTES: &[VanillaAttribute] = &[
    attribute(
        "minecraft:armor",
        "attribute.name.armor",
        VanillaAttributeSentiment::Positive,
    ),
    attribute(
        "minecraft:armor_toughness",
        "attribute.name.armor_toughness",
        VanillaAttributeSentiment::Positive,
    ),
    attribute(
        "minecraft:attack_damage",
        "attribute.name.attack_damage",
        VanillaAttributeSentiment::Positive,
    ),
    attribute(
        "minecraft:attack_knockback",
        "attribute.name.attack_knockback",
        VanillaAttributeSentiment::Positive,
    ),
    attribute(
        "minecraft:attack_speed",
        "attribute.name.attack_speed",
        VanillaAttributeSentiment::Positive,
    ),
    attribute(
        "minecraft:block_break_speed",
        "attribute.name.block_break_speed",
        VanillaAttributeSentiment::Positive,
    ),
    attribute(
        "minecraft:block_interaction_range",
        "attribute.name.block_interaction_range",
        VanillaAttributeSentiment::Positive,
    ),
    attribute(
        "minecraft:burning_time",
        "attribute.name.burning_time",
        VanillaAttributeSentiment::Negative,
    ),
    attribute(
        "minecraft:camera_distance",
        "attribute.name.camera_distance",
        VanillaAttributeSentiment::Positive,
    ),
    attribute(
        "minecraft:explosion_knockback_resistance",
        "attribute.name.explosion_knockback_resistance",
        VanillaAttributeSentiment::Positive,
    ),
    attribute(
        "minecraft:entity_interaction_range",
        "attribute.name.entity_interaction_range",
        VanillaAttributeSentiment::Positive,
    ),
    attribute(
        "minecraft:fall_damage_multiplier",
        "attribute.name.fall_damage_multiplier",
        VanillaAttributeSentiment::Negative,
    ),
    attribute(
        "minecraft:flying_speed",
        "attribute.name.flying_speed",
        VanillaAttributeSentiment::Positive,
    ),
    attribute(
        "minecraft:follow_range",
        "attribute.name.follow_range",
        VanillaAttributeSentiment::Positive,
    ),
    attribute(
        "minecraft:gravity",
        "attribute.name.gravity",
        VanillaAttributeSentiment::Neutral,
    ),
    attribute(
        "minecraft:jump_strength",
        "attribute.name.jump_strength",
        VanillaAttributeSentiment::Positive,
    ),
    attribute(
        "minecraft:knockback_resistance",
        "attribute.name.knockback_resistance",
        VanillaAttributeSentiment::Positive,
    ),
    attribute(
        "minecraft:luck",
        "attribute.name.luck",
        VanillaAttributeSentiment::Positive,
    ),
    attribute(
        "minecraft:max_absorption",
        "attribute.name.max_absorption",
        VanillaAttributeSentiment::Positive,
    ),
    attribute(
        "minecraft:max_health",
        "attribute.name.max_health",
        VanillaAttributeSentiment::Positive,
    ),
    attribute(
        "minecraft:mining_efficiency",
        "attribute.name.mining_efficiency",
        VanillaAttributeSentiment::Positive,
    ),
    attribute(
        "minecraft:movement_efficiency",
        "attribute.name.movement_efficiency",
        VanillaAttributeSentiment::Positive,
    ),
    attribute(
        "minecraft:movement_speed",
        "attribute.name.movement_speed",
        VanillaAttributeSentiment::Positive,
    ),
    attribute(
        "minecraft:oxygen_bonus",
        "attribute.name.oxygen_bonus",
        VanillaAttributeSentiment::Positive,
    ),
    attribute(
        "minecraft:safe_fall_distance",
        "attribute.name.safe_fall_distance",
        VanillaAttributeSentiment::Positive,
    ),
    attribute(
        "minecraft:scale",
        "attribute.name.scale",
        VanillaAttributeSentiment::Neutral,
    ),
    attribute(
        "minecraft:sneaking_speed",
        "attribute.name.sneaking_speed",
        VanillaAttributeSentiment::Positive,
    ),
    attribute(
        "minecraft:spawn_reinforcements",
        "attribute.name.spawn_reinforcements",
        VanillaAttributeSentiment::Positive,
    ),
    attribute(
        "minecraft:step_height",
        "attribute.name.step_height",
        VanillaAttributeSentiment::Positive,
    ),
    attribute(
        "minecraft:submerged_mining_speed",
        "attribute.name.submerged_mining_speed",
        VanillaAttributeSentiment::Positive,
    ),
    attribute(
        "minecraft:sweeping_damage_ratio",
        "attribute.name.sweeping_damage_ratio",
        VanillaAttributeSentiment::Positive,
    ),
    attribute(
        "minecraft:tempt_range",
        "attribute.name.tempt_range",
        VanillaAttributeSentiment::Positive,
    ),
    attribute(
        "minecraft:water_movement_efficiency",
        "attribute.name.water_movement_efficiency",
        VanillaAttributeSentiment::Positive,
    ),
    attribute(
        "minecraft:waypoint_transmit_range",
        "attribute.name.waypoint_transmit_range",
        VanillaAttributeSentiment::Neutral,
    ),
    attribute(
        "minecraft:waypoint_receive_range",
        "attribute.name.waypoint_receive_range",
        VanillaAttributeSentiment::Neutral,
    ),
];

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

pub(super) fn vanilla_attribute(attribute_id: i32) -> Option<VanillaAttribute> {
    usize::try_from(attribute_id)
        .ok()
        .and_then(|index| VANILLA_ATTRIBUTES.get(index).copied())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vanilla_attributes_follow_26_1_bootstrap_order() {
        assert_eq!(VANILLA_ATTRIBUTES.len(), 35);
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
