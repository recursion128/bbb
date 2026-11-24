use crate::{
    decode_direct_registry_json,
    registry_codec::{recursive_values::RegistryCodecDecodeContext, RegistryElementSource},
};
use bbb_protocol::{nbt_summary::entry, NbtSummaryValue};

use super::*;

#[test]
fn leaf_seams_decode_owned_families_and_decline_unowned_ones() {
    let data = decode_direct_registry_json(br#"{"amount":1,"type":"change_item_damage"}"#).unwrap();
    let mut context = RegistryCodecDecodeContext::new(
        "minecraft:enchantment",
        "Enchantment.DIRECT_CODEC",
        RegistryElementSource::ServerDatapackJson,
    );
    context.charge_node("$").unwrap();
    context.charge_node("$.type").unwrap();
    let decoded = decode_entity_leaf_effect_source(
        EnchantmentEntityEffectCodecFamily::ChangeItemDamage,
        "minecraft:change_item_damage".to_string(),
        data.root(),
        "$",
        &mut context,
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        decoded.network_nbt_unchecked(),
        &NbtSummaryValue::Compound(vec![
            entry("amount", NbtSummaryValue::Float(1.0_f32.to_bits())),
            entry(
                "type",
                NbtSummaryValue::String("minecraft:change_item_damage".to_string().into()),
            ),
        ])
    );
    assert!(decode_entity_leaf_effect_source(
        EnchantmentEntityEffectCodecFamily::AllOfEntityEffects,
        "minecraft:all_of".to_string(),
        data.root(),
        "$",
        &mut context,
    )
    .is_none());
    assert!(decode_location_leaf_effect_source(
        EnchantmentLocationBasedEffectCodecFamily::AllOfLocationBasedEffects,
        "minecraft:all_of".to_string(),
        data.root(),
        "$",
        &mut context,
    )
    .is_none());
}
