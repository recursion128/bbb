use bbb_protocol::{
    data_component_types::{
        VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID, VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID,
    },
    item_component_value_types::{ItemUseAnimationSummary, VANILLA_ITEM_USE_ANIMATIONS},
    mob_effect_types::{VANILLA_MOB_EFFECT_SPEED_ID, VANILLA_MOB_EFFECT_WEAKNESS_ID},
    packets::inventory::{ConsumeEffectSummary, DataComponentPatchSummary},
};

use super::super::{
    apply_item_patch_value, decode_supported_component_value, DialogDataComponentValueState,
};
use super::*;

const CONSUMABLE_WHAT: &str = "dialog item component minecraft:consumable";
const DEATH_PROTECTION_WHAT: &str = "dialog item component minecraft:death_protection";

#[test]
fn decodes_consumable_defaults_all_animations_and_context_free_empty_sets() {
    let defaults = decode_consumable(&compound(Vec::new()), None, CONSUMABLE_WHAT).unwrap();
    assert_eq!(defaults.consume_seconds_bits, 1.6f32.to_bits());
    assert_eq!(defaults.animation, ItemUseAnimationSummary::Eat);
    assert!(defaults.has_consume_particles);
    assert!(defaults.on_consume_effects.is_empty());
    assert!(matches!(
        defaults.sound,
        DialogSoundEventHolderState::Named {
            resource_id,
            ..
        } if resource_id == DEFAULT_CONSUME_SOUND
    ));

    for entry in VANILLA_ITEM_USE_ANIMATIONS {
        let value = compound(vec![string_entry(
            "animation",
            entry.serialized_name.unwrap(),
        )]);
        assert_eq!(
            decode_consumable(&value, None, CONSUMABLE_WHAT)
                .unwrap()
                .animation,
            entry.value
        );
    }

    let empty_remove = effect(
        "remove_effects",
        vec![value_entry("effects", NbtSummaryValue::List(Vec::new()))],
    );
    let effects = decode_death_protection(
        &compound(vec![value_entry(
            "death_effects",
            NbtSummaryValue::List(vec![empty_remove]),
        )]),
        None,
        DEATH_PROTECTION_WHAT,
    )
    .unwrap();
    let DialogConsumeEffectState::RemoveEffects { effects } = &effects[0] else {
        panic!("expected remove-effects projection");
    };
    assert_eq!(effects.holder_set_kind, "direct");
    assert!(effects.entries.is_empty());
}

#[test]
fn decodes_all_consume_effect_variants_for_both_components() {
    let effects = all_effects();
    let value = compound(vec![
        value_entry("consume_seconds", float(0.5)),
        string_entry("animation", "drink"),
        value_entry(
            "sound",
            compound(vec![
                string_entry("sound_id", "example:consume"),
                value_entry("range", float(4.0)),
            ]),
        ),
        value_entry("has_consume_particles", NbtSummaryValue::Byte(0)),
        value_entry("on_consume_effects", NbtSummaryValue::List(effects.clone())),
    ]);
    let registries = RegistrySet::default();
    let decoded = decode_supported_component_value(
        VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
        "minecraft:consumable",
        &value,
        Some(&registries),
    )
    .unwrap();
    let Some(DialogDataComponentValueState::Consumable { consumable }) = decoded else {
        panic!("expected consumable component");
    };
    assert_eq!(consumable.consume_seconds_bits, 0.5f32.to_bits());
    assert_eq!(consumable.animation, ItemUseAnimationSummary::Drink);
    assert!(!consumable.has_consume_particles);
    assert!(matches!(
        &consumable.sound,
        DialogSoundEventHolderState::Direct {
            sound_id,
            fixed_range_bits: Some(bits),
        } if sound_id == "example:consume" && *bits == 4.0f32.to_bits()
    ));
    assert_eq!(consumable.on_consume_effects.len(), 5);

    let DialogConsumeEffectState::ApplyEffects {
        effects,
        probability_bits,
    } = &consumable.on_consume_effects[0]
    else {
        panic!("expected apply-effects projection");
    };
    assert_eq!(*probability_bits, 0.25f32.to_bits());
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].effect.holder_id, VANILLA_MOB_EFFECT_SPEED_ID);
    assert_eq!(effects[0].details.duration, 40);
    assert_eq!(effects[0].details.amplifier, 2);

    let DialogConsumeEffectState::RemoveEffects { effects } = &consumable.on_consume_effects[1]
    else {
        panic!("expected remove-effects projection");
    };
    assert_eq!(
        effects
            .entries
            .iter()
            .map(|entry| entry.holder_id)
            .collect::<Vec<_>>(),
        [VANILLA_MOB_EFFECT_SPEED_ID, VANILLA_MOB_EFFECT_WEAKNESS_ID]
    );
    assert!(matches!(
        consumable.on_consume_effects[2],
        DialogConsumeEffectState::ClearAllEffects
    ));
    assert!(matches!(
        consumable.on_consume_effects[3],
        DialogConsumeEffectState::TeleportRandomly { diameter_bits }
            if diameter_bits == 16.0f32.to_bits()
    ));
    assert!(matches!(
        consumable.on_consume_effects[4],
        DialogConsumeEffectState::PlaySound {
            sound: DialogSoundEventHolderState::Named { ref resource_id, .. }
        } if resource_id == DEFAULT_CONSUME_SOUND
    ));

    let death_value = compound(vec![value_entry(
        "death_effects",
        NbtSummaryValue::List(effects_for_death()),
    )]);
    let decoded = decode_supported_component_value(
        VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID,
        "minecraft:death_protection",
        &death_value,
        Some(&registries),
    )
    .unwrap();
    let Some(DialogDataComponentValueState::DeathProtection { effects }) = decoded else {
        panic!("expected death-protection component");
    };
    assert_eq!(effects, consumable.on_consume_effects);

    let mut patch = DataComponentPatchSummary::default();
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
        &DialogDataComponentValueState::Consumable {
            consumable: consumable.clone(),
        },
        &mut patch,
    ));
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID,
        &DialogDataComponentValueState::DeathProtection {
            effects: effects.clone(),
        },
        &mut patch,
    ));
    assert_eq!(
        patch.consumable.as_ref().unwrap().consume_seconds.to_bits(),
        0.5f32.to_bits()
    );
    assert_eq!(
        patch.consumable.as_ref().unwrap().animation,
        ItemUseAnimationSummary::Drink
    );
    assert_eq!(
        patch.consumable_sound.as_ref().unwrap().sound_id.as_deref(),
        Some("example:consume")
    );
    assert_eq!(
        patch.consumable_sound.as_ref().unwrap().fixed_range_bits,
        Some(4.0f32.to_bits())
    );
    assert_eq!(patch.consumable_has_consume_particles, Some(false));
    assert_eq!(patch.consumable_effect_count, Some(5));
    let ConsumeEffectSummary::ApplyEffects {
        effects: applied,
        probability_bits,
    } = &patch.consumable_effects[0]
    else {
        panic!("expected apply-effects summary");
    };
    assert_eq!(*probability_bits, 0.25f32.to_bits());
    assert_eq!(applied[0].effect_id, VANILLA_MOB_EFFECT_SPEED_ID);
    assert_eq!(applied[0].amplifier, 2);
    assert_eq!(applied[0].duration, 40);
    let hidden = applied[0].hidden_effect.as_ref().unwrap();
    assert_eq!(hidden.duration, 5);
    assert!(!hidden.show_particles);
    assert!(!hidden.show_icon);
    let ConsumeEffectSummary::RemoveEffects {
        effect_registry_ids,
        effect_tag,
    } = &patch.consumable_effects[1]
    else {
        panic!("expected remove-effects summary");
    };
    assert_eq!(
        effect_registry_ids,
        &[VANILLA_MOB_EFFECT_SPEED_ID, VANILLA_MOB_EFFECT_WEAKNESS_ID]
    );
    assert_eq!(effect_tag, &None);
    assert_eq!(patch.death_protection_effects, patch.consumable_effects);

    let mut invalid = consumable;
    let DialogConsumeEffectState::RemoveEffects { effects } = &mut invalid.on_consume_effects[1]
    else {
        unreachable!();
    };
    effects.holder_set_kind = "named".to_string();
    effects.tag = None;
    assert!(!apply_item_patch_value(
        VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
        &DialogDataComponentValueState::Consumable {
            consumable: invalid,
        },
        &mut DataComponentPatchSummary::default(),
    ));
}

#[test]
fn rejects_malformed_consume_effect_records_and_codec_ranges() {
    for (value, registries, expected) in [
        (NbtSummaryValue::Int(0), None, "is not a compound"),
        (
            compound(vec![value_entry("consume_seconds", float(-1.0))]),
            None,
            "consume_seconds -1 is outside",
        ),
        (
            compound(vec![value_entry("consume_seconds", float(-0.0))]),
            None,
            "consume_seconds -0 is outside",
        ),
        (
            compound(vec![value_entry("consume_seconds", float(f32::INFINITY))]),
            None,
            "consume_seconds inf is outside",
        ),
        (
            compound(vec![string_entry("animation", "missing")]),
            None,
            "unknown dialog item component minecraft:consumable field animation",
        ),
        (
            compound(vec![value_entry("animation", NbtSummaryValue::Int(0))]),
            None,
            "animation is not a string",
        ),
        (
            compound(vec![value_entry("sound", NbtSummaryValue::Int(0))]),
            None,
            "sound holder is neither",
        ),
        (
            compound(vec![value_entry("has_consume_particles", string("true"))]),
            None,
            "has_consume_particles is not a boolean",
        ),
        (
            component_with_effects(NbtSummaryValue::Int(0)),
            None,
            "on_consume_effects is not a list",
        ),
        (
            component_with_effect(NbtSummaryValue::Int(0)),
            None,
            "on_consume_effects[0] is not a compound",
        ),
        (
            component_with_effect(compound(Vec::new())),
            None,
            "on_consume_effects[0] is missing field type",
        ),
        (
            component_with_effect(compound(vec![value_entry("type", NbtSummaryValue::Int(0))])),
            None,
            "field type is not a resource id",
        ),
        (
            component_with_effect(effect("missing", Vec::new())),
            None,
            "unknown dialog item component minecraft:consumable field on_consume_effects[0] type",
        ),
        (
            component_with_effect(effect("apply_effects", Vec::new())),
            None,
            "missing field effects",
        ),
        (
            component_with_effect(effect(
                "apply_effects",
                vec![value_entry("effects", NbtSummaryValue::Int(0))],
            )),
            None,
            "effects is not a list",
        ),
        (
            component_with_effect(effect(
                "apply_effects",
                vec![
                    value_entry("effects", NbtSummaryValue::List(Vec::new())),
                    value_entry("probability", float(-0.0)),
                ],
            )),
            None,
            "probability -0 is outside",
        ),
        (
            component_with_effect(effect(
                "apply_effects",
                vec![
                    value_entry("effects", NbtSummaryValue::List(Vec::new())),
                    value_entry("probability", float(1.01)),
                ],
            )),
            None,
            "probability 1.01 is outside",
        ),
        (
            component_with_effect(effect("remove_effects", Vec::new())),
            None,
            "missing field effects",
        ),
        (
            component_with_effect(effect(
                "remove_effects",
                vec![value_entry("effects", string("speed"))],
            )),
            None,
            "without registry context",
        ),
        (
            component_with_effect(effect(
                "teleport_randomly",
                vec![value_entry("diameter", float(0.0))],
            )),
            None,
            "diameter 0 is outside",
        ),
        (
            component_with_effect(effect(
                "teleport_randomly",
                vec![value_entry("diameter", float(f32::INFINITY))],
            )),
            None,
            "diameter inf is outside",
        ),
        (
            component_with_effect(effect("play_sound", Vec::new())),
            None,
            "missing field sound",
        ),
    ] {
        let error = decode_consumable(&value, registries, CONSUMABLE_WHAT).unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }

    for (value, expected) in [
        (NbtSummaryValue::Int(0), "is not a compound"),
        (
            compound(vec![value_entry("death_effects", NbtSummaryValue::Int(0))]),
            "death_effects is not a list",
        ),
    ] {
        let error = decode_death_protection(&value, None, DEATH_PROTECTION_WHAT).unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }
}

fn all_effects() -> Vec<NbtSummaryValue> {
    vec![
        effect(
            "apply_effects",
            vec![
                value_entry(
                    "effects",
                    NbtSummaryValue::List(vec![compound(vec![
                        string_entry("id", "speed"),
                        value_entry("amplifier", NbtSummaryValue::Int(2)),
                        value_entry("duration", NbtSummaryValue::Int(40)),
                        value_entry(
                            "hidden_effect",
                            compound(vec![
                                value_entry("duration", NbtSummaryValue::Int(5)),
                                value_entry("show_particles", NbtSummaryValue::Byte(0)),
                            ]),
                        ),
                    ])]),
                ),
                value_entry("probability", float(0.25)),
            ],
        ),
        effect(
            "remove_effects",
            vec![value_entry(
                "effects",
                NbtSummaryValue::List(vec![string("speed"), string("weakness")]),
            )],
        ),
        effect("clear_all_effects", Vec::new()),
        effect("teleport_randomly", Vec::new()),
        effect(
            "play_sound",
            vec![value_entry("sound", string(DEFAULT_CONSUME_SOUND))],
        ),
    ]
}

fn effects_for_death() -> Vec<NbtSummaryValue> {
    all_effects()
}

fn component_with_effect(effect: NbtSummaryValue) -> NbtSummaryValue {
    component_with_effects(NbtSummaryValue::List(vec![effect]))
}

fn component_with_effects(effects: NbtSummaryValue) -> NbtSummaryValue {
    compound(vec![value_entry("on_consume_effects", effects)])
}

fn effect(effect_type: &str, mut entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    entries.insert(0, string_entry("type", effect_type));
    compound(entries)
}

fn float(value: f32) -> NbtSummaryValue {
    NbtSummaryValue::Float(value.to_bits())
}

fn string(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::String(value.to_string())
}

fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(entries)
}

fn string_entry(name: &str, value: &str) -> NbtSummaryEntry {
    value_entry(name, string(value))
}

fn value_entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.to_string(),
        value,
    }
}
