use bbb_protocol::mob_effect_types::{
    self, VANILLA_MOB_EFFECT_ABSORPTION_ID, VANILLA_MOB_EFFECT_HASTE_ID,
    VANILLA_MOB_EFFECT_HEALTH_BOOST_ID, VANILLA_MOB_EFFECT_INVISIBILITY_ID,
    VANILLA_MOB_EFFECT_JUMP_BOOST_ID, VANILLA_MOB_EFFECT_LUCK_ID,
    VANILLA_MOB_EFFECT_MINING_FATIGUE_ID, VANILLA_MOB_EFFECT_SLOWNESS_ID,
    VANILLA_MOB_EFFECT_SPEED_ID, VANILLA_MOB_EFFECT_STRENGTH_ID, VANILLA_MOB_EFFECT_UNLUCK_ID,
    VANILLA_MOB_EFFECT_WEAKNESS_ID,
};

pub(super) use bbb_protocol::mob_effect_types::MobEffectCategory as VanillaMobEffectCategory;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct VanillaMobEffectAttributeModifier {
    pub attribute_description_key: &'static str,
    pub amount: f64,
    pub operation_id: i32,
}

#[derive(Debug, Clone, Copy)]
struct VanillaMobEffect {
    key: &'static str,
    category: VanillaMobEffectCategory,
}

const fn attribute_modifier(
    attribute_description_key: &'static str,
    amount: f64,
    operation_id: i32,
) -> VanillaMobEffectAttributeModifier {
    VanillaMobEffectAttributeModifier {
        attribute_description_key,
        amount,
        operation_id,
    }
}

const SPEED_ATTRIBUTE_MODIFIERS: &[VanillaMobEffectAttributeModifier] =
    &[attribute_modifier("attribute.name.movement_speed", 0.2, 2)];
const SLOWNESS_ATTRIBUTE_MODIFIERS: &[VanillaMobEffectAttributeModifier] = &[attribute_modifier(
    "attribute.name.movement_speed",
    -0.15,
    2,
)];
const HASTE_ATTRIBUTE_MODIFIERS: &[VanillaMobEffectAttributeModifier] =
    &[attribute_modifier("attribute.name.attack_speed", 0.1, 2)];
const MINING_FATIGUE_ATTRIBUTE_MODIFIERS: &[VanillaMobEffectAttributeModifier] =
    &[attribute_modifier("attribute.name.attack_speed", -0.1, 2)];
const STRENGTH_ATTRIBUTE_MODIFIERS: &[VanillaMobEffectAttributeModifier] =
    &[attribute_modifier("attribute.name.attack_damage", 3.0, 0)];
const JUMP_BOOST_ATTRIBUTE_MODIFIERS: &[VanillaMobEffectAttributeModifier] = &[attribute_modifier(
    "attribute.name.safe_fall_distance",
    1.0,
    0,
)];
const INVISIBILITY_ATTRIBUTE_MODIFIERS: &[VanillaMobEffectAttributeModifier] =
    &[attribute_modifier(
        "attribute.name.waypoint_transmit_range",
        -1.0,
        2,
    )];
const WEAKNESS_ATTRIBUTE_MODIFIERS: &[VanillaMobEffectAttributeModifier] =
    &[attribute_modifier("attribute.name.attack_damage", -4.0, 0)];
const HEALTH_BOOST_ATTRIBUTE_MODIFIERS: &[VanillaMobEffectAttributeModifier] =
    &[attribute_modifier("attribute.name.max_health", 4.0, 0)];
const ABSORPTION_ATTRIBUTE_MODIFIERS: &[VanillaMobEffectAttributeModifier] =
    &[attribute_modifier("attribute.name.max_absorption", 4.0, 0)];
const LUCK_ATTRIBUTE_MODIFIERS: &[VanillaMobEffectAttributeModifier] =
    &[attribute_modifier("attribute.name.luck", 1.0, 0)];
const UNLUCK_ATTRIBUTE_MODIFIERS: &[VanillaMobEffectAttributeModifier] =
    &[attribute_modifier("attribute.name.luck", -1.0, 0)];

fn vanilla_mob_effect(effect_id: i32) -> Option<VanillaMobEffect> {
    let entry = mob_effect_types::vanilla_mob_effect_type(effect_id)?;
    Some(VanillaMobEffect {
        key: entry.resource_id,
        category: entry.category,
    })
}

pub fn vanilla_mob_effect_resource_id(effect_id: i32) -> Option<&'static str> {
    vanilla_mob_effect(effect_id).map(|effect| effect.key)
}

pub(super) fn vanilla_mob_effect_key(effect_id: i32) -> Option<&'static str> {
    vanilla_mob_effect_resource_id(effect_id)
}

pub(super) fn vanilla_mob_effect_category(effect_id: i32) -> Option<VanillaMobEffectCategory> {
    vanilla_mob_effect(effect_id).map(|effect| effect.category)
}

pub(super) fn vanilla_mob_effect_attribute_modifiers(
    effect_id: i32,
) -> &'static [VanillaMobEffectAttributeModifier] {
    match effect_id {
        VANILLA_MOB_EFFECT_SPEED_ID => SPEED_ATTRIBUTE_MODIFIERS,
        VANILLA_MOB_EFFECT_SLOWNESS_ID => SLOWNESS_ATTRIBUTE_MODIFIERS,
        VANILLA_MOB_EFFECT_HASTE_ID => HASTE_ATTRIBUTE_MODIFIERS,
        VANILLA_MOB_EFFECT_MINING_FATIGUE_ID => MINING_FATIGUE_ATTRIBUTE_MODIFIERS,
        VANILLA_MOB_EFFECT_STRENGTH_ID => STRENGTH_ATTRIBUTE_MODIFIERS,
        VANILLA_MOB_EFFECT_JUMP_BOOST_ID => JUMP_BOOST_ATTRIBUTE_MODIFIERS,
        VANILLA_MOB_EFFECT_INVISIBILITY_ID => INVISIBILITY_ATTRIBUTE_MODIFIERS,
        VANILLA_MOB_EFFECT_WEAKNESS_ID => WEAKNESS_ATTRIBUTE_MODIFIERS,
        VANILLA_MOB_EFFECT_HEALTH_BOOST_ID => HEALTH_BOOST_ATTRIBUTE_MODIFIERS,
        VANILLA_MOB_EFFECT_ABSORPTION_ID => ABSORPTION_ATTRIBUTE_MODIFIERS,
        VANILLA_MOB_EFFECT_LUCK_ID => LUCK_ATTRIBUTE_MODIFIERS,
        VANILLA_MOB_EFFECT_UNLUCK_ID => UNLUCK_ATTRIBUTE_MODIFIERS,
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vanilla_mob_effect_attribute_modifiers_follow_26_1_mob_effects() {
        assert_eq!(vanilla_mob_effect_resource_id(0), Some("minecraft:speed"));
        assert_eq!(
            vanilla_mob_effect_resource_id(39),
            Some("minecraft:breath_of_the_nautilus")
        );
        assert_eq!(vanilla_mob_effect_resource_id(40), None);
        assert_eq!(
            vanilla_mob_effect_category(0),
            Some(VanillaMobEffectCategory::Beneficial)
        );
        assert_eq!(
            vanilla_mob_effect_category(18),
            Some(VanillaMobEffectCategory::Harmful)
        );
        assert_eq!(
            vanilla_mob_effect_category(30),
            Some(VanillaMobEffectCategory::Neutral)
        );
        assert_eq!(
            vanilla_mob_effect_attribute_modifiers(0),
            &[attribute_modifier("attribute.name.movement_speed", 0.2, 2)]
        );
        assert_eq!(
            vanilla_mob_effect_attribute_modifiers(17),
            &[attribute_modifier("attribute.name.attack_damage", -4.0, 0)]
        );
        assert_eq!(
            vanilla_mob_effect_attribute_modifiers(26),
            &[attribute_modifier("attribute.name.luck", -1.0, 0)]
        );
        assert_eq!(vanilla_mob_effect_attribute_modifiers(18), &[]);
        assert_eq!(vanilla_mob_effect_attribute_modifiers(-1), &[]);
    }
}
