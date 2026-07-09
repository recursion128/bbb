#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VanillaMobEffectCategory {
    Beneficial,
    Harmful,
    Neutral,
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
