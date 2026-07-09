#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VanillaMobEffectCategory {
    Beneficial,
    Harmful,
    Neutral,
}

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

const VANILLA_MOB_EFFECTS: &[VanillaMobEffect] = &[
    effect("minecraft:speed", VanillaMobEffectCategory::Beneficial),
    effect("minecraft:slowness", VanillaMobEffectCategory::Harmful),
    effect("minecraft:haste", VanillaMobEffectCategory::Beneficial),
    effect(
        "minecraft:mining_fatigue",
        VanillaMobEffectCategory::Harmful,
    ),
    effect("minecraft:strength", VanillaMobEffectCategory::Beneficial),
    effect(
        "minecraft:instant_health",
        VanillaMobEffectCategory::Beneficial,
    ),
    effect(
        "minecraft:instant_damage",
        VanillaMobEffectCategory::Harmful,
    ),
    effect("minecraft:jump_boost", VanillaMobEffectCategory::Beneficial),
    effect("minecraft:nausea", VanillaMobEffectCategory::Harmful),
    effect(
        "minecraft:regeneration",
        VanillaMobEffectCategory::Beneficial,
    ),
    effect("minecraft:resistance", VanillaMobEffectCategory::Beneficial),
    effect(
        "minecraft:fire_resistance",
        VanillaMobEffectCategory::Beneficial,
    ),
    effect(
        "minecraft:water_breathing",
        VanillaMobEffectCategory::Beneficial,
    ),
    effect(
        "minecraft:invisibility",
        VanillaMobEffectCategory::Beneficial,
    ),
    effect("minecraft:blindness", VanillaMobEffectCategory::Harmful),
    effect(
        "minecraft:night_vision",
        VanillaMobEffectCategory::Beneficial,
    ),
    effect("minecraft:hunger", VanillaMobEffectCategory::Harmful),
    effect("minecraft:weakness", VanillaMobEffectCategory::Harmful),
    effect("minecraft:poison", VanillaMobEffectCategory::Harmful),
    effect("minecraft:wither", VanillaMobEffectCategory::Harmful),
    effect(
        "minecraft:health_boost",
        VanillaMobEffectCategory::Beneficial,
    ),
    effect("minecraft:absorption", VanillaMobEffectCategory::Beneficial),
    effect("minecraft:saturation", VanillaMobEffectCategory::Beneficial),
    effect("minecraft:glowing", VanillaMobEffectCategory::Neutral),
    effect("minecraft:levitation", VanillaMobEffectCategory::Harmful),
    effect("minecraft:luck", VanillaMobEffectCategory::Beneficial),
    effect("minecraft:unluck", VanillaMobEffectCategory::Harmful),
    effect(
        "minecraft:slow_falling",
        VanillaMobEffectCategory::Beneficial,
    ),
    effect(
        "minecraft:conduit_power",
        VanillaMobEffectCategory::Beneficial,
    ),
    effect(
        "minecraft:dolphins_grace",
        VanillaMobEffectCategory::Beneficial,
    ),
    effect("minecraft:bad_omen", VanillaMobEffectCategory::Neutral),
    effect(
        "minecraft:hero_of_the_village",
        VanillaMobEffectCategory::Beneficial,
    ),
    effect("minecraft:darkness", VanillaMobEffectCategory::Harmful),
    effect("minecraft:trial_omen", VanillaMobEffectCategory::Neutral),
    effect("minecraft:raid_omen", VanillaMobEffectCategory::Neutral),
    effect("minecraft:wind_charged", VanillaMobEffectCategory::Harmful),
    effect("minecraft:weaving", VanillaMobEffectCategory::Harmful),
    effect("minecraft:oozing", VanillaMobEffectCategory::Harmful),
    effect("minecraft:infested", VanillaMobEffectCategory::Harmful),
    effect(
        "minecraft:breath_of_the_nautilus",
        VanillaMobEffectCategory::Beneficial,
    ),
];

const fn effect(key: &'static str, category: VanillaMobEffectCategory) -> VanillaMobEffect {
    VanillaMobEffect { key, category }
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
    let effect_id = usize::try_from(effect_id).ok()?;
    VANILLA_MOB_EFFECTS.get(effect_id).copied()
}

pub(super) fn vanilla_mob_effect_key(effect_id: i32) -> Option<&'static str> {
    vanilla_mob_effect(effect_id).map(|effect| effect.key)
}

pub(super) fn vanilla_mob_effect_category(effect_id: i32) -> Option<VanillaMobEffectCategory> {
    vanilla_mob_effect(effect_id).map(|effect| effect.category)
}

pub(super) fn vanilla_mob_effect_attribute_modifiers(
    effect_id: i32,
) -> &'static [VanillaMobEffectAttributeModifier] {
    match effect_id {
        0 => SPEED_ATTRIBUTE_MODIFIERS,
        1 => SLOWNESS_ATTRIBUTE_MODIFIERS,
        2 => HASTE_ATTRIBUTE_MODIFIERS,
        3 => MINING_FATIGUE_ATTRIBUTE_MODIFIERS,
        4 => STRENGTH_ATTRIBUTE_MODIFIERS,
        7 => JUMP_BOOST_ATTRIBUTE_MODIFIERS,
        13 => INVISIBILITY_ATTRIBUTE_MODIFIERS,
        17 => WEAKNESS_ATTRIBUTE_MODIFIERS,
        20 => HEALTH_BOOST_ATTRIBUTE_MODIFIERS,
        21 => ABSORPTION_ATTRIBUTE_MODIFIERS,
        25 => LUCK_ATTRIBUTE_MODIFIERS,
        26 => UNLUCK_ATTRIBUTE_MODIFIERS,
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vanilla_mob_effect_attribute_modifiers_follow_26_1_mob_effects() {
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
