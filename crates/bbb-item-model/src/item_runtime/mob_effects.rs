use bbb_protocol::mob_effect_types::{self, MobEffectAttributeModifierDescriptor};

pub(super) use bbb_protocol::mob_effect_types::MobEffectCategory as VanillaMobEffectCategory;

#[derive(Debug, Clone, Copy)]
struct VanillaMobEffect {
    key: &'static str,
    category: VanillaMobEffectCategory,
}

fn vanilla_mob_effect(effect_id: i32) -> Option<VanillaMobEffect> {
    let entry = mob_effect_types::vanilla_mob_effect_type(effect_id)?;
    Some(VanillaMobEffect {
        key: entry.resource_id,
        category: entry.category,
    })
}

pub(super) fn vanilla_mob_effect_key(effect_id: i32) -> Option<&'static str> {
    vanilla_mob_effect(effect_id).map(|effect| effect.key)
}

pub(super) fn vanilla_mob_effect_category(effect_id: i32) -> Option<VanillaMobEffectCategory> {
    vanilla_mob_effect(effect_id).map(|effect| effect.category)
}

pub(super) fn vanilla_mob_effect_attribute_modifiers(
    effect_id: i32,
) -> &'static [MobEffectAttributeModifierDescriptor] {
    mob_effect_types::vanilla_mob_effect_type(effect_id)
        .map_or(&[], |effect| effect.attribute_modifiers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vanilla_mob_effect_attribute_modifiers_follow_26_1_mob_effects() {
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
        let speed = vanilla_mob_effect_attribute_modifiers(0)[0];
        assert_eq!(speed.attribute_id, 22);
        assert_eq!(f64::from_bits(speed.amount_bits), f64::from(0.2_f32));
        assert_eq!(speed.operation_id, 2);
        let weakness = vanilla_mob_effect_attribute_modifiers(17)[0];
        assert_eq!(weakness.attribute_id, 2);
        assert_eq!(f64::from_bits(weakness.amount_bits), -4.0);
        let unluck = vanilla_mob_effect_attribute_modifiers(26)[0];
        assert_eq!(unluck.attribute_id, 17);
        assert_eq!(f64::from_bits(unluck.amount_bits), -1.0);
        assert_eq!(vanilla_mob_effect_attribute_modifiers(18), &[]);
        assert_eq!(vanilla_mob_effect_attribute_modifiers(-1), &[]);
    }
}
