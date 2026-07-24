use std::collections::BTreeMap;

use bbb_protocol::builtin_enum_component_values::{
    VANILLA_DYE_COLOR_BLUE_ID, VANILLA_DYE_COLOR_LIGHT_BLUE_ID, VANILLA_DYE_COLOR_PURPLE_ID,
    VANILLA_DYE_COLOR_RED_ID, VANILLA_DYE_COLOR_YELLOW_ID, VANILLA_HORSE_VARIANT_DARK_BROWN_ID,
    VANILLA_LLAMA_VARIANT_GRAY_ID, VANILLA_RABBIT_VARIANT_EVIL_ID,
};
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID, VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID,
    VANILLA_DATA_COMPONENT_CREATIVE_SLOT_LOCK_TYPE_ID,
    VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID,
    VANILLA_DATA_COMPONENT_DYE_TYPE_ID, VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
    VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID, VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
    VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID, VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID,
    VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
    VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
    VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID,
    VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID, VANILLA_DATA_COMPONENT_TRIM_TYPE_ID,
    VANILLA_DATA_COMPONENT_TROPICAL_FISH_BASE_COLOR_TYPE_ID,
    VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_COLOR_TYPE_ID,
    VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID,
};
use bbb_protocol::mob_effect_types::{
    VANILLA_MOB_EFFECT_HASTE_ID, VANILLA_MOB_EFFECT_POISON_ID, VANILLA_MOB_EFFECT_SLOWNESS_ID,
    VANILLA_MOB_EFFECT_SPEED_ID,
};

use super::trim::{DialogTrimMaterialState, DialogTrimPatternState};
use super::*;

#[test]
fn decodes_primitive_component_value_codecs() {
    for (type_id, resource_id, input, expected) in [
        (
            VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
            "minecraft:max_stack_size",
            NbtSummaryValue::Byte(99),
            DialogDataComponentValueState::Integer { value: 99 },
        ),
        (
            VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
            "minecraft:max_damage",
            NbtSummaryValue::Long(10),
            DialogDataComponentValueState::Integer { value: 10 },
        ),
        (
            VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
            "minecraft:damage",
            NbtSummaryValue::Double(3.75f64.to_bits()),
            DialogDataComponentValueState::Integer { value: 3 },
        ),
        (
            VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID,
            "minecraft:repair_cost",
            NbtSummaryValue::Short(7),
            DialogDataComponentValueState::Integer { value: 7 },
        ),
        (
            VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID,
            "minecraft:enchantment_glint_override",
            NbtSummaryValue::Int(2),
            DialogDataComponentValueState::Boolean { value: true },
        ),
    ] {
        assert_eq!(
            decoded(type_id, resource_id, &input).unwrap(),
            Some(expected)
        );
    }

    for (type_id, resource_id) in [
        (
            VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID,
            "minecraft:unbreakable",
        ),
        (
            VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID,
            "minecraft:intangible_projectile",
        ),
        (VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID, "minecraft:glider"),
    ] {
        assert_eq!(
            decoded(type_id, resource_id, &compound(Vec::new())).unwrap(),
            Some(DialogDataComponentValueState::Unit)
        );
    }

    let minimum = decoded(
        VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
        "minecraft:minimum_attack_charge",
        &NbtSummaryValue::Double(0.5f64.to_bits()),
    )
    .unwrap()
    .unwrap();
    assert_eq!(minimum.float_value(), Some(0.5));
    let scale = decoded(
        VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
        "minecraft:potion_duration_scale",
        &NbtSummaryValue::Double(1.25f64.to_bits()),
    )
    .unwrap()
    .unwrap();
    assert_eq!(scale.float_value(), Some(1.25));

    for (type_id, resource_id, raw, normalized) in [
        (
            VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID,
            "minecraft:item_model",
            "stone",
            "minecraft:stone",
        ),
        (
            VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID,
            "minecraft:tooltip_style",
            ":compact",
            "minecraft:compact",
        ),
        (
            VANILLA_DATA_COMPONENT_NOTE_BLOCK_SOUND_TYPE_ID,
            "minecraft:note_block_sound",
            "block.note_block.harp",
            "minecraft:block.note_block.harp",
        ),
    ] {
        assert_eq!(
            decoded(
                type_id,
                resource_id,
                &NbtSummaryValue::String(raw.to_string())
            )
            .unwrap(),
            Some(DialogDataComponentValueState::ResourceId {
                resource_id: normalized.to_string(),
            })
        );
    }
    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_RARITY_TYPE_ID,
            "minecraft:rarity",
            &NbtSummaryValue::String("rare".to_string()),
        )
        .unwrap(),
        Some(DialogDataComponentValueState::Rarity {
            value: ItemRaritySummary::Rare,
        })
    );
}

#[test]
fn rejects_invalid_primitive_component_value_codecs() {
    for (type_id, resource_id, value, expected) in [
        (
            VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
            "minecraft:max_stack_size",
            NbtSummaryValue::Int(0),
            "outside 1..=99",
        ),
        (
            VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
            "minecraft:max_damage",
            NbtSummaryValue::Int(0),
            "outside 1..=",
        ),
        (
            VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
            "minecraft:damage",
            NbtSummaryValue::Int(-1),
            "outside 0..=",
        ),
        (
            VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
            "minecraft:minimum_attack_charge",
            NbtSummaryValue::Float((-0.0f32).to_bits()),
            "outside 0..=1",
        ),
        (
            VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
            "minecraft:potion_duration_scale",
            NbtSummaryValue::Float(f32::INFINITY.to_bits()),
            "outside 0..=",
        ),
        (
            VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID,
            "minecraft:enchantment_glint_override",
            NbtSummaryValue::String("true".to_string()),
            "is not a boolean",
        ),
        (
            VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID,
            "minecraft:unbreakable",
            NbtSummaryValue::Byte(1),
            "is not a compound",
        ),
        (
            VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID,
            "minecraft:item_model",
            NbtSummaryValue::String("Invalid".to_string()),
            "invalid dialog item component",
        ),
        (
            VANILLA_DATA_COMPONENT_RARITY_TYPE_ID,
            "minecraft:rarity",
            NbtSummaryValue::String("legendary".to_string()),
            "unknown dialog item component",
        ),
    ] {
        let error = decoded(type_id, resource_id, &value).unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }
}

#[test]
fn decodes_all_string_enum_component_value_codecs() {
    let expected = [
        ("dye", "red", 14),
        ("base_color", "blue", 11),
        ("wolf/collar", "lime", 5),
        ("fox/variant", "snow", 1),
        ("salmon/size", "large", 2),
        ("parrot/variant", "gray", 4),
        ("tropical_fish/pattern", "spotty", 1280),
        ("tropical_fish/base_color", "cyan", 9),
        ("tropical_fish/pattern_color", "purple", 10),
        ("mooshroom/variant", "brown", 1),
        ("rabbit/variant", "evil", 99),
        ("horse/variant", "dark_brown", 6),
        ("llama/variant", "gray", 3),
        ("axolotl/variant", "blue", 4),
        ("cat/collar", "orange", 1),
        ("sheep/color", "white", 0),
        ("shulker/color", "black", 15),
    ];

    for (component, serialized_name, value_id) in expected {
        let resource_id = format!("minecraft:{component}");
        let type_id =
            bbb_protocol::data_component_types::vanilla_data_component_type_for_resource_id(
                &resource_id,
            )
            .unwrap()
            .type_id;
        assert_eq!(
            decoded(
                type_id,
                &resource_id,
                &NbtSummaryValue::String(serialized_name.to_string()),
            )
            .unwrap(),
            Some(DialogDataComponentValueState::StringEnum {
                value_id,
                serialized_name: serialized_name.to_string(),
            })
        );
    }
}

#[test]
fn tropical_fish_pattern_color_string_enum_projects_to_owned_patch_field() {
    let mut patch = DataComponentPatchSummary::default();
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_COLOR_TYPE_ID,
        &DialogDataComponentValueState::StringEnum {
            value_id: VANILLA_DYE_COLOR_PURPLE_ID,
            serialized_name: "purple".to_string(),
        },
        &mut patch,
    ));
    assert_eq!(
        patch.tropical_fish_pattern_color_id,
        Some(VANILLA_DYE_COLOR_PURPLE_ID)
    );
    assert!(patch.builtin_enum_component_ids.is_empty());
}

#[test]
fn tropical_fish_base_color_string_enum_projects_to_owned_patch_field() {
    let mut patch = DataComponentPatchSummary::default();
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_BASE_COLOR_TYPE_ID,
        &DialogDataComponentValueState::StringEnum {
            value_id: VANILLA_DYE_COLOR_BLUE_ID,
            serialized_name: "blue".to_string(),
        },
        &mut patch,
    ));
    assert_eq!(
        patch.tropical_fish_base_color_id,
        Some(VANILLA_DYE_COLOR_BLUE_ID)
    );
    assert!(patch.builtin_enum_component_ids.is_empty());
}

#[test]
fn rabbit_variant_string_enum_projects_to_builtin_enum_patch_map() {
    let mut patch = DataComponentPatchSummary::default();
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID,
        &DialogDataComponentValueState::StringEnum {
            value_id: VANILLA_RABBIT_VARIANT_EVIL_ID,
            serialized_name: "evil".to_string(),
        },
        &mut patch,
    ));
    assert_eq!(
        patch.builtin_enum_component_ids,
        BTreeMap::from([(
            VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID,
            VANILLA_RABBIT_VARIANT_EVIL_ID,
        )])
    );
}

#[test]
fn horse_variant_string_enum_projects_to_builtin_enum_patch_map() {
    let mut patch = DataComponentPatchSummary::default();
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID,
        &DialogDataComponentValueState::StringEnum {
            value_id: VANILLA_HORSE_VARIANT_DARK_BROWN_ID,
            serialized_name: "dark_brown".to_string(),
        },
        &mut patch,
    ));
    assert_eq!(
        patch.builtin_enum_component_ids,
        BTreeMap::from([(
            VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID,
            VANILLA_HORSE_VARIANT_DARK_BROWN_ID,
        )])
    );
}

#[test]
fn llama_variant_string_enum_projects_to_builtin_enum_patch_map() {
    let mut patch = DataComponentPatchSummary::default();
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID,
        &DialogDataComponentValueState::StringEnum {
            value_id: VANILLA_LLAMA_VARIANT_GRAY_ID,
            serialized_name: "gray".to_string(),
        },
        &mut patch,
    ));
    assert_eq!(
        patch.builtin_enum_component_ids,
        BTreeMap::from([(
            VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID,
            VANILLA_LLAMA_VARIANT_GRAY_ID,
        )])
    );
}

#[test]
fn rejects_invalid_string_enum_component_value_codecs() {
    for (type_id, resource_id, value, expected) in [
        (
            VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
            "minecraft:dye",
            NbtSummaryValue::String("scarlet".to_string()),
            "unknown dialog item component minecraft:dye enum value scarlet",
        ),
        (
            VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID,
            "minecraft:fox/variant",
            NbtSummaryValue::Int(1),
            "is not an enum name",
        ),
        (
            VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID,
            "minecraft:tropical_fish/pattern",
            NbtSummaryValue::String("missing".to_string()),
            "unknown dialog item component minecraft:tropical_fish/pattern enum value missing",
        ),
    ] {
        let error = decoded(type_id, resource_id, &value).unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }
}

#[test]
fn decodes_scalar_wrappers_flat_records_and_string_map() {
    for (type_id, resource_id, input, expected) in [
        (
            VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID,
            "minecraft:map_color",
            NbtSummaryValue::Long(0x1_0012_3456),
            DialogDataComponentValueState::Integer { value: 0x0012_3456 },
        ),
        (
            VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID,
            "minecraft:map_id",
            NbtSummaryValue::Double(12.75f64.to_bits()),
            DialogDataComponentValueState::Integer { value: 12 },
        ),
        (
            VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID,
            "minecraft:ominous_bottle_amplifier",
            NbtSummaryValue::Byte(4),
            DialogDataComponentValueState::Integer { value: 4 },
        ),
    ] {
        assert_eq!(
            decoded(type_id, resource_id, &input).unwrap(),
            Some(expected)
        );
    }

    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
            "minecraft:use_effects",
            &compound(Vec::new()),
        )
        .unwrap(),
        Some(DialogDataComponentValueState::UseEffects {
            can_sprint: false,
            interact_vibrations: true,
            speed_multiplier_bits: 0.2f32.to_bits(),
        })
    );
    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
            "minecraft:use_effects",
            &compound(vec![
                value_entry("can_sprint", NbtSummaryValue::Int(2)),
                value_entry("interact_vibrations", NbtSummaryValue::Short(256)),
                value_entry(
                    "speed_multiplier",
                    NbtSummaryValue::Double(0.75f64.to_bits()),
                ),
            ]),
        )
        .unwrap(),
        Some(DialogDataComponentValueState::UseEffects {
            can_sprint: true,
            interact_vibrations: false,
            speed_multiplier_bits: 0.75f32.to_bits(),
        })
    );

    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
            "minecraft:swing_animation",
            &compound(Vec::new()),
        )
        .unwrap(),
        Some(DialogDataComponentValueState::SwingAnimation {
            animation_type: SwingAnimationTypeSummary::Whack,
            duration: 6,
        })
    );
    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
            "minecraft:swing_animation",
            &compound(vec![
                string_entry("type", "stab"),
                value_entry("duration", NbtSummaryValue::Long(9)),
            ]),
        )
        .unwrap(),
        Some(DialogDataComponentValueState::SwingAnimation {
            animation_type: SwingAnimationTypeSummary::Stab,
            duration: 9,
        })
    );

    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID,
            "minecraft:block_state",
            &compound(vec![
                string_entry("axis", "x"),
                string_entry("waterlogged", "false"),
            ]),
        )
        .unwrap(),
        Some(DialogDataComponentValueState::BlockState {
            properties: vec![
                DialogStringMapEntryState {
                    key: "axis".to_string(),
                    value: "x".to_string(),
                },
                DialogStringMapEntryState {
                    key: "waterlogged".to_string(),
                    value: "false".to_string(),
                },
            ],
        })
    );
}

#[test]
fn rejects_invalid_scalar_wrapper_and_flat_record_codecs() {
    for (type_id, resource_id, value, expected) in [
        (
            VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID,
            "minecraft:ominous_bottle_amplifier",
            NbtSummaryValue::Int(5),
            "outside 0..=4",
        ),
        (
            VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
            "minecraft:use_effects",
            NbtSummaryValue::Byte(1),
            "is not a compound",
        ),
        (
            VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
            "minecraft:use_effects",
            compound(vec![value_entry(
                "speed_multiplier",
                NbtSummaryValue::Float((-0.0f32).to_bits()),
            )]),
            "outside 0..=1",
        ),
        (
            VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
            "minecraft:swing_animation",
            compound(vec![string_entry("type", "slash")]),
            "unknown dialog item component minecraft:swing_animation animation type slash",
        ),
        (
            VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
            "minecraft:swing_animation",
            compound(vec![value_entry("duration", NbtSummaryValue::Int(0))]),
            "outside 1..=",
        ),
        (
            VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID,
            "minecraft:block_state",
            compound(vec![value_entry("axis", NbtSummaryValue::Int(1))]),
            "property axis is not a string",
        ),
    ] {
        let error = decoded(type_id, resource_id, &value).unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }
}

#[test]
fn decodes_numeric_item_behavior_records() {
    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
            "minecraft:food",
            &compound(vec![
                value_entry("nutrition", NbtSummaryValue::Long(4)),
                value_entry("saturation", NbtSummaryValue::Float(f32::NAN.to_bits())),
                value_entry("can_always_eat", NbtSummaryValue::Int(2)),
            ]),
        )
        .unwrap(),
        Some(DialogDataComponentValueState::Food {
            nutrition: 4,
            saturation_bits: f32::NAN.to_bits(),
            can_always_eat: true,
        })
    );

    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
            "minecraft:use_cooldown",
            &compound(vec![
                value_entry("seconds", NbtSummaryValue::Double(1.25f64.to_bits())),
                string_entry("cooldown_group", ":ender_pearl"),
            ]),
        )
        .unwrap(),
        Some(DialogDataComponentValueState::UseCooldown {
            seconds_bits: 1.25f32.to_bits(),
            cooldown_group: Some("minecraft:ender_pearl".to_string()),
        })
    );

    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID,
            "minecraft:weapon",
            &compound(Vec::new()),
        )
        .unwrap(),
        Some(DialogDataComponentValueState::Weapon {
            item_damage_per_attack: 1,
            disable_blocking_for_seconds_bits: 0.0f32.to_bits(),
        })
    );
    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID,
            "minecraft:weapon",
            &compound(vec![
                value_entry(
                    "item_damage_per_attack",
                    NbtSummaryValue::Double(3.75f64.to_bits())
                ),
                value_entry("disable_blocking_for_seconds", NbtSummaryValue::Long(5)),
            ]),
        )
        .unwrap(),
        Some(DialogDataComponentValueState::Weapon {
            item_damage_per_attack: 3,
            disable_blocking_for_seconds_bits: 5.0f32.to_bits(),
        })
    );

    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID,
            "minecraft:attack_range",
            &compound(Vec::new()),
        )
        .unwrap(),
        Some(DialogDataComponentValueState::AttackRange {
            min_reach_bits: 0.0f32.to_bits(),
            max_reach_bits: 3.0f32.to_bits(),
            min_creative_reach_bits: 0.0f32.to_bits(),
            max_creative_reach_bits: 5.0f32.to_bits(),
            hitbox_margin_bits: 0.3f32.to_bits(),
            mob_factor_bits: 1.0f32.to_bits(),
        })
    );
    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID,
            "minecraft:attack_range",
            &compound(vec![
                value_entry("min_reach", NbtSummaryValue::Byte(1)),
                value_entry("max_reach", NbtSummaryValue::Long(64)),
                value_entry("min_creative_reach", NbtSummaryValue::Short(2)),
                value_entry(
                    "max_creative_reach",
                    NbtSummaryValue::Float(7.5f32.to_bits())
                ),
                value_entry("hitbox_margin", NbtSummaryValue::Double(0.75f64.to_bits())),
                value_entry("mob_factor", NbtSummaryValue::Int(2)),
            ]),
        )
        .unwrap(),
        Some(DialogDataComponentValueState::AttackRange {
            min_reach_bits: 1.0f32.to_bits(),
            max_reach_bits: 64.0f32.to_bits(),
            min_creative_reach_bits: 2.0f32.to_bits(),
            max_creative_reach_bits: 7.5f32.to_bits(),
            hitbox_margin_bits: 0.75f32.to_bits(),
            mob_factor_bits: 2.0f32.to_bits(),
        })
    );

    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID,
            "minecraft:enchantable",
            &compound(vec![value_entry(
                "value",
                NbtSummaryValue::Double(12.75f64.to_bits()),
            )]),
        )
        .unwrap(),
        Some(DialogDataComponentValueState::Enchantable { value: 12 })
    );
}

#[test]
fn rejects_invalid_numeric_item_behavior_records() {
    for (type_id, resource_id, value, expected) in [
        (
            VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
            "minecraft:food",
            compound(vec![value_entry("nutrition", NbtSummaryValue::Int(1))]),
            "missing field saturation",
        ),
        (
            VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
            "minecraft:food",
            compound(vec![
                value_entry("nutrition", NbtSummaryValue::Int(-1)),
                value_entry("saturation", NbtSummaryValue::Float(0.0f32.to_bits())),
            ]),
            "field nutrition -1 is outside 0..=",
        ),
        (
            VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
            "minecraft:use_cooldown",
            compound(vec![value_entry(
                "seconds",
                NbtSummaryValue::Float((-0.0f32).to_bits()),
            )]),
            "field seconds -0 is outside 0..=",
        ),
        (
            VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
            "minecraft:use_cooldown",
            compound(vec![
                value_entry("seconds", NbtSummaryValue::Float(1.0f32.to_bits())),
                string_entry("cooldown_group", "Invalid"),
            ]),
            "invalid dialog item component minecraft:use_cooldown cooldown group",
        ),
        (
            VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID,
            "minecraft:weapon",
            compound(vec![value_entry(
                "item_damage_per_attack",
                NbtSummaryValue::Int(-1),
            )]),
            "field item_damage_per_attack -1 is outside 0..=",
        ),
        (
            VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID,
            "minecraft:weapon",
            compound(vec![value_entry(
                "disable_blocking_for_seconds",
                NbtSummaryValue::Float((-0.0f32).to_bits()),
            )]),
            "field disable_blocking_for_seconds -0 is outside 0..=",
        ),
        (
            VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID,
            "minecraft:attack_range",
            compound(vec![value_entry(
                "max_reach",
                NbtSummaryValue::Double(64.25f64.to_bits()),
            )]),
            "field max_reach 64.25 is outside 0..=64",
        ),
        (
            VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID,
            "minecraft:attack_range",
            compound(vec![value_entry(
                "mob_factor",
                NbtSummaryValue::Float(f32::NAN.to_bits()),
            )]),
            "field mob_factor NaN is outside 0..=2",
        ),
        (
            VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID,
            "minecraft:enchantable",
            compound(vec![value_entry("value", NbtSummaryValue::Int(0))]),
            "field value 0 is outside 1..=",
        ),
    ] {
        let error = decoded(type_id, resource_id, &value).unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }
}

#[test]
fn decodes_resource_key_and_component_catalog_metadata() {
    let recipes = DialogDataComponentValueState::Recipes {
        resource_ids: vec![
            "minecraft:stick".to_string(),
            "minecraft:custom".to_string(),
            "minecraft:stick".to_string(),
        ],
    };
    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID,
            "minecraft:recipes",
            &NbtSummaryValue::List(vec![
                NbtSummaryValue::String("stick".to_string()),
                NbtSummaryValue::String(":custom".to_string()),
                NbtSummaryValue::String("stick".to_string()),
            ]),
        )
        .unwrap(),
        Some(recipes.clone())
    );

    let mut patch = DataComponentPatchSummary::default();
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID,
        &recipes,
        &mut patch,
    ));
    assert_eq!(
        patch
            .codec_backed_nbt
            .get(&VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID),
        Some(&NbtSummaryValue::List(vec![
            NbtSummaryValue::String("minecraft:stick".to_string()),
            NbtSummaryValue::String("minecraft:custom".to_string()),
            NbtSummaryValue::String("minecraft:stick".to_string()),
        ]))
    );

    let mut empty_patch = DataComponentPatchSummary::default();
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID,
        &DialogDataComponentValueState::Recipes {
            resource_ids: Vec::new(),
        },
        &mut empty_patch,
    ));
    assert_eq!(
        empty_patch
            .codec_backed_nbt
            .get(&VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID),
        Some(&NbtSummaryValue::List(Vec::new()))
    );

    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID,
            "minecraft:tooltip_display",
            &compound(Vec::new()),
        )
        .unwrap(),
        Some(DialogDataComponentValueState::TooltipDisplay {
            hide_tooltip: false,
            hidden_components: Vec::new(),
        })
    );
    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID,
            "minecraft:tooltip_display",
            &compound(vec![
                value_entry("hide_tooltip", NbtSummaryValue::Int(2)),
                value_entry(
                    "hidden_components",
                    NbtSummaryValue::List(vec![
                        NbtSummaryValue::String("custom_name".to_string()),
                        NbtSummaryValue::String("creative_slot_lock".to_string()),
                        NbtSummaryValue::String("minecraft:custom_name".to_string()),
                    ]),
                ),
            ]),
        )
        .unwrap(),
        Some(DialogDataComponentValueState::TooltipDisplay {
            hide_tooltip: true,
            hidden_components: vec![
                DialogDataComponentReferenceState {
                    type_id: VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
                    resource_id: "minecraft:custom_name".to_string(),
                },
                DialogDataComponentReferenceState {
                    type_id: VANILLA_DATA_COMPONENT_CREATIVE_SLOT_LOCK_TYPE_ID,
                    resource_id: "minecraft:creative_slot_lock".to_string(),
                },
            ],
        })
    );

    let container_loot = decoded(
        VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID,
        "minecraft:container_loot",
        &compound(vec![
            string_entry("loot_table", "chests/simple_dungeon"),
            value_entry("seed", NbtSummaryValue::Double(12.75f64.to_bits())),
        ]),
    )
    .unwrap();
    assert_eq!(
        container_loot,
        Some(DialogDataComponentValueState::ContainerLoot {
            loot_table: "minecraft:chests/simple_dungeon".to_string(),
            seed: 12,
        })
    );
    let default_seed_loot = decoded(
        VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID,
        "minecraft:container_loot",
        &compound(vec![string_entry("loot_table", ":empty")]),
    )
    .unwrap();
    assert_eq!(
        default_seed_loot,
        Some(DialogDataComponentValueState::ContainerLoot {
            loot_table: "minecraft:empty".to_string(),
            seed: 0,
        })
    );

    let mut patch = DataComponentPatchSummary::default();
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID,
        container_loot.as_ref().unwrap(),
        &mut patch,
    ));
    assert!(patch.container_loot);
    assert_eq!(
        patch
            .codec_backed_nbt
            .get(&VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID),
        Some(&compound(vec![
            string_entry("loot_table", "minecraft:chests/simple_dungeon"),
            value_entry("seed", NbtSummaryValue::Long(12)),
        ]))
    );

    let mut default_seed_patch = DataComponentPatchSummary::default();
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID,
        default_seed_loot.as_ref().unwrap(),
        &mut default_seed_patch,
    ));
    assert!(default_seed_patch.container_loot);
    assert_eq!(
        default_seed_patch
            .codec_backed_nbt
            .get(&VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID),
        Some(&compound(vec![string_entry(
            "loot_table",
            "minecraft:empty"
        )]))
    );
}

#[test]
fn rejects_invalid_resource_key_and_component_catalog_metadata() {
    for (type_id, resource_id, value, expected) in [
        (
            VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID,
            "minecraft:recipes",
            compound(Vec::new()),
            "is not a list",
        ),
        (
            VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID,
            "minecraft:recipes",
            NbtSummaryValue::List(vec![NbtSummaryValue::Int(1)]),
            "[0] is not a resource id",
        ),
        (
            VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID,
            "minecraft:recipes",
            NbtSummaryValue::List(vec![NbtSummaryValue::String("Invalid".to_string())]),
            "invalid dialog item component minecraft:recipes[0] resource id Invalid",
        ),
        (
            VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID,
            "minecraft:tooltip_display",
            compound(vec![value_entry(
                "hidden_components",
                NbtSummaryValue::String("custom_name".to_string()),
            )]),
            "field hidden_components is not a list",
        ),
        (
            VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID,
            "minecraft:tooltip_display",
            compound(vec![value_entry(
                "hidden_components",
                NbtSummaryValue::List(vec![NbtSummaryValue::String("missing".to_string())]),
            )]),
            "unknown dialog item component minecraft:tooltip_display hidden component minecraft:missing",
        ),
        (
            VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID,
            "minecraft:container_loot",
            compound(Vec::new()),
            "missing field loot_table",
        ),
        (
            VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID,
            "minecraft:container_loot",
            compound(vec![string_entry("loot_table", "Invalid")]),
            "invalid dialog item component minecraft:container_loot loot table Invalid",
        ),
        (
            VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID,
            "minecraft:container_loot",
            compound(vec![
                string_entry("loot_table", "chests/simple_dungeon"),
                string_entry("seed", "twelve"),
            ]),
            "field seed is not numeric",
        ),
    ] {
        let error = decoded(type_id, resource_id, &value).unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }
}

#[test]
fn decodes_rgb_alternatives_and_custom_model_data_lists() {
    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID,
            "minecraft:dyed_color",
            &NbtSummaryValue::Long(0x1_0012_3456),
        )
        .unwrap(),
        Some(DialogDataComponentValueState::RgbColor { value: 0x0012_3456 })
    );
    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID,
            "minecraft:dyed_color",
            &float_vector(&[0.5, 1.0, 0.25]),
        )
        .unwrap(),
        Some(DialogDataComponentValueState::RgbColor {
            value: 0xff7f_ff3fu32 as i32,
        })
    );
    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID,
            "minecraft:dyed_color",
            &NbtSummaryValue::IntArray(vec![0, 1, 0]),
        )
        .unwrap(),
        Some(DialogDataComponentValueState::RgbColor {
            value: 0xff00_ff00u32 as i32,
        })
    );

    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID,
            "minecraft:custom_model_data",
            &compound(Vec::new()),
        )
        .unwrap(),
        Some(DialogDataComponentValueState::CustomModelData {
            floats_bits: Vec::new(),
            flags: Vec::new(),
            strings: Vec::new(),
            colors: Vec::new(),
        })
    );

    let out_of_range_color = NbtSummaryValue::List(vec![
        NbtSummaryValue::Float((-0.1f32).to_bits()),
        NbtSummaryValue::Float(1.1f32.to_bits()),
        NbtSummaryValue::Float(f32::NAN.to_bits()),
    ]);
    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID,
            "minecraft:custom_model_data",
            &compound(vec![
                value_entry(
                    "floats",
                    NbtSummaryValue::List(vec![
                        NbtSummaryValue::Float(1.25f32.to_bits()),
                        NbtSummaryValue::Float(f32::NAN.to_bits()),
                    ]),
                ),
                value_entry(
                    "flags",
                    NbtSummaryValue::List(
                        vec![NbtSummaryValue::Int(2), NbtSummaryValue::Int(256),]
                    ),
                ),
                value_entry(
                    "strings",
                    NbtSummaryValue::List(vec![
                        NbtSummaryValue::String("first".to_string()),
                        NbtSummaryValue::String("second".to_string()),
                    ]),
                ),
                value_entry(
                    "colors",
                    NbtSummaryValue::List(vec![
                        float_vector(&[0.5, 1.0, 0.25]),
                        out_of_range_color,
                    ]),
                ),
            ]),
        )
        .unwrap(),
        Some(DialogDataComponentValueState::CustomModelData {
            floats_bits: vec![1.25f32.to_bits(), f32::NAN.to_bits()],
            flags: vec![true, false],
            strings: vec!["first".to_string(), "second".to_string()],
            colors: vec![0xff7f_ff3fu32 as i32, 0xffe6_1800u32 as i32],
        })
    );
    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID,
            "minecraft:custom_model_data",
            &compound(vec![
                value_entry("floats", NbtSummaryValue::IntArray(vec![1, -2])),
                value_entry("flags", NbtSummaryValue::ByteArray(vec![0, 2])),
                value_entry("colors", NbtSummaryValue::LongArray(vec![0x11_22_33])),
            ]),
        )
        .unwrap(),
        Some(DialogDataComponentValueState::CustomModelData {
            floats_bits: vec![1.0f32.to_bits(), (-2.0f32).to_bits()],
            flags: vec![false, true],
            strings: Vec::new(),
            colors: vec![0x11_22_33],
        })
    );
}

#[test]
fn rejects_invalid_rgb_alternatives_and_custom_model_data_lists() {
    for (type_id, resource_id, value, expected) in [
        (
            VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID,
            "minecraft:dyed_color",
            float_vector(&[0.0, 1.0]),
            "vector has 2 channels instead of 3",
        ),
        (
            VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID,
            "minecraft:dyed_color",
            NbtSummaryValue::List(vec![
                NbtSummaryValue::Float(0.0f32.to_bits()),
                NbtSummaryValue::String("green".to_string()),
                NbtSummaryValue::Float(1.0f32.to_bits()),
            ]),
            "vector[1] is not numeric",
        ),
        (
            VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID,
            "minecraft:custom_model_data",
            NbtSummaryValue::List(Vec::new()),
            "is not a compound",
        ),
        (
            VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID,
            "minecraft:custom_model_data",
            compound(vec![value_entry("floats", NbtSummaryValue::Int(1))]),
            "field floats is not a list",
        ),
        (
            VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID,
            "minecraft:custom_model_data",
            compound(vec![value_entry(
                "flags",
                NbtSummaryValue::List(vec![NbtSummaryValue::String("true".to_string())]),
            )]),
            "field flags[0] is not a boolean",
        ),
        (
            VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID,
            "minecraft:custom_model_data",
            compound(vec![value_entry(
                "strings",
                NbtSummaryValue::List(vec![NbtSummaryValue::Int(1)]),
            )]),
            "field strings[0] is not a string",
        ),
        (
            VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID,
            "minecraft:custom_model_data",
            compound(vec![value_entry(
                "colors",
                NbtSummaryValue::List(vec![float_vector(&[0.0, 0.5, 1.0, 0.5])]),
            )]),
            "field colors[0] vector has 4 channels instead of 3",
        ),
    ] {
        let error = decoded(type_id, resource_id, &value).unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }
}

#[test]
fn decodes_firework_component_records_and_collection_tags() {
    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
            "minecraft:firework_explosion",
            &firework_explosion("star"),
        )
        .unwrap(),
        Some(DialogDataComponentValueState::FireworkExplosion {
            explosion: FireworkExplosionSummary {
                shape: FireworkExplosionShapeSummary::Star,
                colors: Vec::new(),
                fade_colors: Vec::new(),
                has_trail: false,
                has_twinkle: false,
            },
        })
    );

    let detailed = compound(vec![
        string_entry("shape", "creeper"),
        value_entry("colors", NbtSummaryValue::IntArray(vec![0x11_22_33, -1])),
        value_entry(
            "fade_colors",
            NbtSummaryValue::LongArray(vec![0x1_0044_5566]),
        ),
        value_entry("has_trail", NbtSummaryValue::Int(2)),
        value_entry("has_twinkle", NbtSummaryValue::Int(256)),
    ]);
    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
            "minecraft:firework_explosion",
            &detailed,
        )
        .unwrap(),
        Some(DialogDataComponentValueState::FireworkExplosion {
            explosion: FireworkExplosionSummary {
                shape: FireworkExplosionShapeSummary::Creeper,
                colors: vec![0x11_22_33, -1],
                fade_colors: vec![0x44_55_66],
                has_trail: true,
                has_twinkle: false,
            },
        })
    );

    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
            "minecraft:fireworks",
            &compound(Vec::new()),
        )
        .unwrap(),
        Some(DialogDataComponentValueState::Fireworks {
            flight_duration: 0,
            explosions: Vec::new(),
        })
    );
    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
            "minecraft:fireworks",
            &compound(vec![value_entry(
                "flight_duration",
                NbtSummaryValue::Int(256),
            )]),
        )
        .unwrap(),
        Some(DialogDataComponentValueState::Fireworks {
            flight_duration: 0,
            explosions: Vec::new(),
        })
    );
    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
            "minecraft:fireworks",
            &compound(vec![
                value_entry("flight_duration", NbtSummaryValue::Int(-1)),
                value_entry(
                    "explosions",
                    NbtSummaryValue::List(vec![detailed, firework_explosion("burst")]),
                ),
            ]),
        )
        .unwrap(),
        Some(DialogDataComponentValueState::Fireworks {
            flight_duration: 255,
            explosions: vec![
                FireworkExplosionSummary {
                    shape: FireworkExplosionShapeSummary::Creeper,
                    colors: vec![0x11_22_33, -1],
                    fade_colors: vec![0x44_55_66],
                    has_trail: true,
                    has_twinkle: false,
                },
                FireworkExplosionSummary {
                    shape: FireworkExplosionShapeSummary::Burst,
                    colors: Vec::new(),
                    fade_colors: Vec::new(),
                    has_trail: false,
                    has_twinkle: false,
                },
            ],
        })
    );

    let maximum = NbtSummaryValue::List(vec![firework_explosion("small_ball"); 256]);
    let decoded = decoded(
        VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
        "minecraft:fireworks",
        &compound(vec![value_entry("explosions", maximum)]),
    )
    .unwrap()
    .unwrap();
    let DialogDataComponentValueState::Fireworks { explosions, .. } = decoded else {
        panic!("unexpected decoded component {decoded:?}");
    };
    assert_eq!(explosions.len(), 256);
}

#[test]
fn rejects_invalid_firework_component_records() {
    for (type_id, resource_id, value, expected) in [
        (
            VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
            "minecraft:firework_explosion",
            NbtSummaryValue::List(Vec::new()),
            "is not a compound",
        ),
        (
            VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
            "minecraft:firework_explosion",
            compound(Vec::new()),
            "is missing field shape",
        ),
        (
            VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
            "minecraft:firework_explosion",
            compound(vec![string_entry("shape", "sphere")]),
            "unknown dialog item component minecraft:firework_explosion shape sphere",
        ),
        (
            VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
            "minecraft:firework_explosion",
            compound(vec![value_entry("shape", NbtSummaryValue::Int(2))]),
            "field shape is not an enum name",
        ),
        (
            VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
            "minecraft:firework_explosion",
            compound(vec![
                string_entry("shape", "star"),
                value_entry("colors", NbtSummaryValue::String("red".to_string())),
            ]),
            "field colors is not a list",
        ),
        (
            VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
            "minecraft:firework_explosion",
            compound(vec![
                string_entry("shape", "star"),
                value_entry(
                    "fade_colors",
                    NbtSummaryValue::List(vec![NbtSummaryValue::String("blue".to_string())]),
                ),
            ]),
            "field fade_colors[0] is not numeric",
        ),
        (
            VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
            "minecraft:fireworks",
            compound(vec![string_entry("flight_duration", "one")]),
            "field flight_duration is not numeric",
        ),
        (
            VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
            "minecraft:fireworks",
            compound(vec![value_entry("explosions", NbtSummaryValue::Int(1))]),
            "field explosions is not a list",
        ),
        (
            VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
            "minecraft:fireworks",
            compound(vec![value_entry(
                "explosions",
                NbtSummaryValue::List(vec![NbtSummaryValue::Int(1)]),
            )]),
            "field explosions[0] is not a compound",
        ),
        (
            VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
            "minecraft:fireworks",
            compound(vec![value_entry(
                "explosions",
                NbtSummaryValue::List(vec![firework_explosion("small_ball"); 257]),
            )]),
            "field explosions has 257 entries, maximum is 256",
        ),
    ] {
        let error = decoded(type_id, resource_id, &value).unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }
}

#[test]
fn decodes_lodestone_tracker_global_position_and_defaults() {
    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID,
            "minecraft:lodestone_tracker",
            &compound(Vec::new()),
        )
        .unwrap(),
        Some(DialogDataComponentValueState::LodestoneTracker {
            target: None,
            tracked: true,
        })
    );
    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID,
            "minecraft:lodestone_tracker",
            &compound(vec![
                value_entry(
                    "target",
                    compound(vec![
                        string_entry("dimension", "the_nether"),
                        value_entry("pos", NbtSummaryValue::IntArray(vec![12, -64, 34])),
                    ]),
                ),
                value_entry("tracked", NbtSummaryValue::Int(0)),
            ]),
        )
        .unwrap(),
        Some(DialogDataComponentValueState::LodestoneTracker {
            target: Some(LodestoneTargetSummary {
                dimension: "minecraft:the_nether".to_string(),
                pos: BlockPos {
                    x: 12,
                    y: -64,
                    z: 34,
                },
            }),
            tracked: false,
        })
    );
    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID,
            "minecraft:lodestone_tracker",
            &compound(vec![value_entry(
                "target",
                compound(vec![
                    string_entry("dimension", "minecraft:overworld"),
                    value_entry(
                        "pos",
                        NbtSummaryValue::List(vec![
                            NbtSummaryValue::Long(0x1_0000_0001),
                            NbtSummaryValue::Double(2.75f64.to_bits()),
                            NbtSummaryValue::Byte(-3),
                        ]),
                    ),
                ]),
            )]),
        )
        .unwrap(),
        Some(DialogDataComponentValueState::LodestoneTracker {
            target: Some(LodestoneTargetSummary {
                dimension: "minecraft:overworld".to_string(),
                pos: BlockPos { x: 1, y: 2, z: -3 },
            }),
            tracked: true,
        })
    );
}

#[test]
fn rejects_invalid_lodestone_tracker_global_positions() {
    for (value, expected) in [
        (NbtSummaryValue::List(Vec::new()), "is not a compound"),
        (
            compound(vec![value_entry("target", NbtSummaryValue::Int(1))]),
            "field target is not a compound",
        ),
        (
            compound(vec![value_entry("target", compound(Vec::new()))]),
            "field target is missing field dimension",
        ),
        (
            compound(vec![value_entry(
                "target",
                compound(vec![
                    value_entry("dimension", NbtSummaryValue::Int(0)),
                    value_entry("pos", NbtSummaryValue::IntArray(vec![0, 0, 0])),
                ]),
            )]),
            "field dimension is not a resource id",
        ),
        (
            compound(vec![value_entry(
                "target",
                compound(vec![
                    string_entry("dimension", "Invalid"),
                    value_entry("pos", NbtSummaryValue::IntArray(vec![0, 0, 0])),
                ]),
            )]),
            "invalid dialog item component minecraft:lodestone_tracker field target dimension resource id Invalid",
        ),
        (
            compound(vec![value_entry(
                "target",
                compound(vec![string_entry("dimension", "overworld")]),
            )]),
            "field target is missing field pos",
        ),
        (
            compound(vec![value_entry(
                "target",
                compound(vec![
                    string_entry("dimension", "overworld"),
                    value_entry("pos", NbtSummaryValue::Int(0)),
                ]),
            )]),
            "field pos is not an int stream",
        ),
        (
            compound(vec![value_entry(
                "target",
                compound(vec![
                    string_entry("dimension", "overworld"),
                    value_entry("pos", NbtSummaryValue::IntArray(vec![0, 1])),
                ]),
            )]),
            "field pos has 2 coordinates instead of 3",
        ),
        (
            compound(vec![value_entry(
                "target",
                compound(vec![
                    string_entry("dimension", "overworld"),
                    value_entry(
                        "pos",
                        NbtSummaryValue::List(vec![
                            NbtSummaryValue::Int(0),
                            NbtSummaryValue::String("one".to_string()),
                            NbtSummaryValue::Int(2),
                        ]),
                    ),
                ]),
            )]),
            "field pos[1] is not numeric",
        ),
        (
            compound(vec![string_entry("tracked", "true")]),
            "field tracked is not a boolean",
        ),
    ] {
        let error = decoded(
            VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID,
            "minecraft:lodestone_tracker",
            &value,
        )
        .unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }
}

#[test]
fn decodes_provides_banner_patterns_from_session_and_context_free_inputs() {
    let registry = "minecraft:banner_pattern";
    let mut registries = RegistrySet::default();
    let mut contents = crate::RegistryContentState::new(registry);
    contents.entries = vec![
        crate::RegistryPacketEntry::stub("example:first"),
        crate::RegistryPacketEntry::stub("minecraft:flower"),
    ];
    registries.contents.insert(registry.to_string(), contents);

    let decoded = decode_supported_component_value(
        VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
        "minecraft:provides_banner_patterns",
        &NbtSummaryValue::String("minecraft:flower".to_string()),
        Some(&registries),
    )
    .unwrap();
    let Some(DialogDataComponentValueState::ProvidesBannerPatterns { patterns }) = decoded else {
        panic!("expected provides-banner-patterns component");
    };
    assert_eq!(patterns.holder_set_kind, "direct");
    assert_eq!(patterns.entries.len(), 1);
    assert_eq!(patterns.entries[0].holder_id, 1);
    assert_eq!(patterns.entries[0].resource_id, "minecraft:flower");

    let context_free = decode_supported_component_value(
        VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
        "minecraft:provides_banner_patterns",
        &NbtSummaryValue::List(Vec::new()),
        None,
    )
    .unwrap();
    let Some(DialogDataComponentValueState::ProvidesBannerPatterns { patterns }) = context_free
    else {
        panic!("expected context-free provides-banner-patterns component");
    };
    assert!(patterns.entries.is_empty());
}

#[test]
fn decodes_banner_pattern_layers_with_named_holders_and_dye_colors() {
    let registry = "minecraft:banner_pattern";
    let mut registries = RegistrySet::default();
    let mut contents = crate::RegistryContentState::new(registry);
    contents.entries = vec![
        crate::RegistryPacketEntry::stub("example:first"),
        crate::RegistryPacketEntry::stub("minecraft:flower"),
    ];
    registries.contents.insert(registry.to_string(), contents);
    let value = NbtSummaryValue::List(vec![
        compound(vec![
            string_entry("pattern", "minecraft:flower"),
            string_entry("color", "red"),
        ]),
        compound(vec![
            string_entry("pattern", "example:first"),
            string_entry("color", "light_blue"),
        ]),
    ]);
    let decoded = decode_supported_component_value(
        VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
        "minecraft:banner_patterns",
        &value,
        Some(&registries),
    )
    .unwrap();
    let Some(DialogDataComponentValueState::BannerPatterns { layers }) = decoded else {
        panic!("expected banner-patterns component");
    };
    assert_eq!(layers.len(), 2);
    assert!(matches!(
        layers[0].pattern,
        DialogBannerPatternHolderState::Named {
            registry_id: 1,
            ref resource_id,
        } if resource_id == "minecraft:flower"
    ));
    assert_eq!(layers[0].color_id, VANILLA_DYE_COLOR_RED_ID);
    assert_eq!(layers[0].color, "red");
    assert!(matches!(
        layers[1].pattern,
        DialogBannerPatternHolderState::Named { registry_id: 0, .. }
    ));
    assert_eq!(layers[1].color_id, VANILLA_DYE_COLOR_LIGHT_BLUE_ID);

    let direct = decode_supported_component_value(
        VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
        "minecraft:banner_patterns",
        &NbtSummaryValue::List(vec![compound(vec![
            value_entry(
                "pattern",
                compound(vec![
                    string_entry("asset_id", "example:spiral"),
                    string_entry("translation_key", ""),
                    value_entry("ignored", NbtSummaryValue::Int(1)),
                ]),
            ),
            string_entry("color", "blue"),
        ])]),
        None,
    )
    .unwrap();
    let Some(DialogDataComponentValueState::BannerPatterns { layers }) = direct else {
        panic!("expected direct banner-pattern layer");
    };
    assert!(matches!(
        layers.as_slice(),
        [DialogBannerPatternLayerState {
            pattern: DialogBannerPatternHolderState::Direct {
                pattern: DialogDirectBannerPatternState {
                    asset_id,
                    translation_key,
                },
            },
            color_id: VANILLA_DYE_COLOR_BLUE_ID,
            color,
        }] if asset_id == "example:spiral" && translation_key.is_empty() && color == "blue"
    ));

    let context_free = decode_supported_component_value(
        VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
        "minecraft:banner_patterns",
        &NbtSummaryValue::List(Vec::new()),
        None,
    )
    .unwrap();
    assert!(matches!(
        context_free,
        Some(DialogDataComponentValueState::BannerPatterns { layers }) if layers.is_empty()
    ));

    for (value, context, expected) in [
        (compound(Vec::new()), Some(&registries), "is not a list"),
        (
            NbtSummaryValue::List(vec![NbtSummaryValue::String("flower".to_string())]),
            Some(&registries),
            "[0] is not a compound",
        ),
        (
            NbtSummaryValue::List(vec![compound(vec![string_entry("color", "red")])]),
            Some(&registries),
            "missing field pattern",
        ),
        (
            NbtSummaryValue::List(vec![compound(vec![string_entry(
                "pattern",
                "minecraft:flower",
            )])]),
            Some(&registries),
            "missing field color",
        ),
        (
            NbtSummaryValue::List(vec![compound(vec![
                string_entry("pattern", "minecraft:flower"),
                string_entry("color", "chartreuse"),
            ])]),
            Some(&registries),
            "invalid",
        ),
        (
            NbtSummaryValue::List(vec![compound(vec![
                value_entry(
                    "pattern",
                    compound(vec![string_entry("asset_id", "example:flower")]),
                ),
                string_entry("color", "red"),
            ])]),
            Some(&registries),
            "missing field translation_key",
        ),
        (
            NbtSummaryValue::List(vec![compound(vec![
                value_entry(
                    "pattern",
                    compound(vec![
                        string_entry("asset_id", "Bad"),
                        string_entry("translation_key", "block.bad"),
                    ]),
                ),
                string_entry("color", "red"),
            ])]),
            None,
            "invalid",
        ),
        (
            NbtSummaryValue::List(vec![compound(vec![
                value_entry(
                    "pattern",
                    compound(vec![
                        string_entry("asset_id", "example:flower"),
                        value_entry("translation_key", NbtSummaryValue::Int(1)),
                    ]),
                ),
                string_entry("color", "red"),
            ])]),
            None,
            "translation_key is not a string",
        ),
        (
            NbtSummaryValue::List(vec![compound(vec![
                value_entry("pattern", NbtSummaryValue::List(Vec::new())),
                string_entry("color", "red"),
            ])]),
            None,
            "neither a resource id",
        ),
        (value, None, "without registry context"),
    ] {
        let error = decode_supported_component_value(
            VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
            "minecraft:banner_patterns",
            &value,
            context,
        )
        .unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }
}

#[test]
fn decodes_jukebox_playable_from_session_registry() {
    let registry = "minecraft:jukebox_song";
    let mut registries = RegistrySet::default();
    let mut contents = crate::RegistryContentState::new(registry);
    contents.entries = vec![
        crate::RegistryPacketEntry::stub("example:first"),
        crate::RegistryPacketEntry::stub("minecraft:cat"),
    ];
    registries.contents.insert(registry.to_string(), contents);

    let decoded = decode_supported_component_value(
        VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
        "minecraft:jukebox_playable",
        &NbtSummaryValue::String("minecraft:cat".to_string()),
        Some(&registries),
    )
    .unwrap();
    let Some(DialogDataComponentValueState::JukeboxPlayable { song }) = decoded else {
        panic!("expected jukebox-playable component");
    };
    assert_eq!(song.holder_id, 1);
    assert_eq!(song.resource_id, "minecraft:cat");

    assert!(decode_supported_component_value(
        VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
        "minecraft:jukebox_playable",
        &NbtSummaryValue::String("minecraft:cat".to_string()),
        None,
    )
    .unwrap_err()
    .contains("without registry context"));
    assert!(decode_supported_component_value(
        VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
        "minecraft:jukebox_playable",
        &compound(vec![string_entry("description", "not persistent")]),
        Some(&registries),
    )
    .unwrap_err()
    .contains("holder is not a resource id"));
}

#[test]
fn decodes_instrument_and_provides_trim_material_named_holders() {
    let mut registries = RegistrySet::default();
    for (registry, resource) in [
        ("minecraft:instrument", "minecraft:ponder_goat_horn"),
        ("minecraft:trim_material", "minecraft:quartz"),
    ] {
        let mut contents = crate::RegistryContentState::new(registry);
        contents.entries = vec![
            crate::RegistryPacketEntry::stub("example:first"),
            crate::RegistryPacketEntry::stub(resource),
        ];
        registries.contents.insert(registry.to_string(), contents);
    }

    let instrument = decode_supported_component_value(
        VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
        "minecraft:instrument",
        &NbtSummaryValue::String("ponder_goat_horn".to_string()),
        Some(&registries),
    )
    .unwrap();
    assert!(matches!(
        instrument,
        Some(DialogDataComponentValueState::Instrument {
            instrument: DialogInstrumentHolderState::Named {
                registry_id: 1,
                ref resource_id,
            }
        }) if resource_id == "minecraft:ponder_goat_horn"
    ));

    let material = decode_supported_component_value(
        VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
        "minecraft:provides_trim_material",
        &NbtSummaryValue::String("minecraft:quartz".to_string()),
        Some(&registries),
    )
    .unwrap();
    assert!(matches!(
        material,
        Some(DialogDataComponentValueState::ProvidesTrimMaterial {
            material: DialogTrimMaterialHolderState::Named {
                registry_id: 1,
                ref resource_id,
            }
        }) if resource_id == "minecraft:quartz"
    ));

    for (type_id, resource_id, value, context, expected) in [
        (
            VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
            "minecraft:instrument",
            NbtSummaryValue::String("minecraft:ponder_goat_horn".to_string()),
            None,
            "without registry context",
        ),
        (
            VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
            "minecraft:instrument",
            NbtSummaryValue::String("minecraft:missing".to_string()),
            Some(&registries),
            "missing minecraft:instrument resource",
        ),
        (
            VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
            "minecraft:provides_trim_material",
            compound(vec![string_entry("asset_name", "quartz")]),
            Some(&registries),
            "missing field description",
        ),
    ] {
        let error =
            decode_supported_component_value(type_id, resource_id, &value, context).unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }
}

#[test]
fn decodes_trim_named_holder_pair_from_session_registries() {
    let mut registries = RegistrySet::default();
    for (registry, resource) in [
        ("minecraft:trim_material", "minecraft:quartz"),
        ("minecraft:trim_pattern", "minecraft:coast"),
    ] {
        let mut contents = crate::RegistryContentState::new(registry);
        contents.entries = vec![
            crate::RegistryPacketEntry::stub("example:first"),
            crate::RegistryPacketEntry::stub(resource),
        ];
        registries.contents.insert(registry.to_string(), contents);
    }
    let value = compound(vec![
        string_entry("material", "quartz"),
        string_entry("pattern", "minecraft:coast"),
    ]);
    let decoded = decode_supported_component_value(
        VANILLA_DATA_COMPONENT_TRIM_TYPE_ID,
        "minecraft:trim",
        &value,
        Some(&registries),
    )
    .unwrap();
    let Some(DialogDataComponentValueState::Trim { material, pattern }) = decoded else {
        panic!("expected trim component");
    };
    assert!(matches!(
        material,
        DialogTrimMaterialHolderState::Named {
            registry_id: 1,
            ref resource_id,
        } if resource_id == "minecraft:quartz"
    ));
    assert!(matches!(
        pattern,
        DialogTrimPatternHolderState::Named {
            registry_id: 1,
            ref resource_id,
        } if resource_id == "minecraft:coast"
    ));

    for (value, context, expected) in [
        (value.clone(), None, "without registry context"),
        (
            compound(vec![string_entry("pattern", "minecraft:coast")]),
            Some(&registries),
            "missing field material",
        ),
        (
            compound(vec![string_entry("material", "minecraft:quartz")]),
            Some(&registries),
            "missing field pattern",
        ),
        (
            compound(vec![
                value_entry(
                    "material",
                    compound(vec![string_entry("asset_name", "quartz")]),
                ),
                string_entry("pattern", "minecraft:coast"),
            ]),
            Some(&registries),
            "field material direct holder is missing field description",
        ),
        (
            compound(vec![
                string_entry("material", "minecraft:quartz"),
                string_entry("pattern", "minecraft:missing"),
            ]),
            Some(&registries),
            "missing minecraft:trim_pattern resource",
        ),
    ] {
        let error = decode_supported_component_value(
            VANILLA_DATA_COMPONENT_TRIM_TYPE_ID,
            "minecraft:trim",
            &value,
            context,
        )
        .unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }
}

#[test]
fn decodes_all_synchronized_registry_fixed_variant_components() {
    let variants =
        bbb_protocol::data_component_types::vanilla_data_component_animal_variant_registries()
            .map(|(type_id, registry)| {
                (
                    type_id,
                    bbb_protocol::data_component_types::vanilla_data_component_type(type_id)
                        .unwrap()
                        .resource_id,
                    registry,
                )
            })
            .collect::<Vec<_>>();
    let mut registries = RegistrySet::default();
    for &(_, _, registry) in &variants {
        let mut contents = crate::RegistryContentState::new(registry);
        contents.entries = vec![
            crate::RegistryPacketEntry::stub("example:first"),
            crate::RegistryPacketEntry::stub("example:selected"),
        ];
        registries.contents.insert(registry.to_string(), contents);
    }

    for (type_id, resource_id, registry) in variants {
        let decoded = decode_supported_component_value(
            type_id,
            resource_id,
            &NbtSummaryValue::String("example:selected".to_string()),
            Some(&registries),
        )
        .unwrap();
        assert!(matches!(
            decoded,
            Some(DialogDataComponentValueState::RegistryVariant {
                registry: decoded_registry,
                holder,
            }) if decoded_registry == registry
                && holder.holder_id == 1
                && holder.resource_id == "example:selected"
        ));
    }

    for (value, context, expected) in [
        (
            NbtSummaryValue::String("example:selected".to_string()),
            None,
            "without registry context",
        ),
        (
            NbtSummaryValue::String("example:missing".to_string()),
            Some(&registries),
            "missing minecraft:wolf_variant resource",
        ),
        (
            compound(vec![string_entry("texture", "example:wolf")]),
            Some(&registries),
            "holder is not a resource id",
        ),
    ] {
        let error = decode_supported_component_value(
            VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID,
            "minecraft:wolf/variant",
            &value,
            context,
        )
        .unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }
}

#[test]
fn decodes_painting_variant_named_holder_from_session_registry() {
    let registry = "minecraft:painting_variant";
    let mut registries = RegistrySet::default();
    let mut contents = crate::RegistryContentState::new(registry);
    contents.entries = vec![
        crate::RegistryPacketEntry::stub("example:first"),
        crate::RegistryPacketEntry::stub("minecraft:kebab"),
    ];
    registries.contents.insert(registry.to_string(), contents);

    let decoded = decode_supported_component_value(
        VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID,
        "minecraft:painting/variant",
        &NbtSummaryValue::String("kebab".to_string()),
        Some(&registries),
    )
    .unwrap();
    let Some(DialogDataComponentValueState::PaintingVariant { variant }) = decoded else {
        panic!("expected painting-variant component");
    };
    assert_eq!(variant.holder_id, 1);
    assert_eq!(variant.resource_id, "minecraft:kebab");

    for (value, registries, expected) in [
        (
            NbtSummaryValue::String("minecraft:kebab".to_string()),
            None,
            "without registry context",
        ),
        (
            NbtSummaryValue::String("minecraft:missing".to_string()),
            Some(&registries),
            "missing minecraft:painting_variant resource",
        ),
        (
            compound(vec![value_entry("width", NbtSummaryValue::Int(1))]),
            Some(&registries),
            "holder is not a resource id",
        ),
    ] {
        let error = decode_supported_component_value(
            VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID,
            "minecraft:painting/variant",
            &value,
            registries,
        )
        .unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }
}

#[test]
fn decodes_damage_resistant_holder_set_from_session_and_context_free_inputs() {
    let registry = "minecraft:damage_type";
    let mut registries = RegistrySet::default();
    let mut contents = crate::RegistryContentState::new(registry);
    contents.entries = vec![
        crate::RegistryPacketEntry::stub("minecraft:in_fire"),
        crate::RegistryPacketEntry::stub("minecraft:explosion"),
    ];
    registries.contents.insert(registry.to_string(), contents);
    registries.tags.insert(
        registry.to_string(),
        crate::RegistryTagState {
            registry: registry.to_string(),
            tags: std::collections::BTreeMap::from([("minecraft:is_fire".to_string(), vec![0])]),
        },
    );

    let decoded = decode_supported_component_value(
        VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID,
        "minecraft:damage_resistant",
        &compound(vec![string_entry("types", "#minecraft:is_fire")]),
        Some(&registries),
    )
    .unwrap();
    let Some(DialogDataComponentValueState::DamageResistant { types }) = decoded else {
        panic!("expected damage-resistant component");
    };
    assert_eq!(types.holder_set_kind, "named");
    assert_eq!(types.tag.as_deref(), Some("minecraft:is_fire"));
    assert_eq!(types.entries[0].resource_id, "minecraft:in_fire");

    let context_free = decode_supported_component_value(
        VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID,
        "minecraft:damage_resistant",
        &compound(vec![value_entry(
            "types",
            NbtSummaryValue::List(Vec::new()),
        )]),
        None,
    )
    .unwrap();
    assert!(matches!(
        context_free,
        Some(DialogDataComponentValueState::DamageResistant { types })
            if types.entries.is_empty()
    ));

    assert!(decode_supported_component_value(
        VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID,
        "minecraft:damage_resistant",
        &compound(Vec::new()),
        Some(&registries),
    )
    .unwrap_err()
    .contains("missing field types"));
}

#[test]
fn decodes_suspicious_stew_effects_with_builtin_holders_and_lenient_duration() {
    let value = NbtSummaryValue::List(vec![
        compound(vec![string_entry("id", "speed")]),
        compound(vec![
            string_entry("id", "minecraft:poison"),
            value_entry("duration", NbtSummaryValue::Short(-20)),
        ]),
        compound(vec![
            string_entry("id", "haste"),
            string_entry("duration", "invalid"),
        ]),
    ]);
    let decoded = decode_supported_component_value(
        VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID,
        "minecraft:suspicious_stew_effects",
        &value,
        None,
    )
    .unwrap();
    let Some(DialogDataComponentValueState::SuspiciousStewEffects { effects }) = decoded else {
        panic!("expected suspicious-stew-effects component");
    };
    assert_eq!(effects.len(), 3);
    assert_eq!(effects[0].effect.holder_id, VANILLA_MOB_EFFECT_SPEED_ID);
    assert_eq!(effects[0].effect.resource_id, "minecraft:speed");
    assert_eq!(effects[0].duration, 160);
    assert_eq!(effects[1].effect.holder_id, VANILLA_MOB_EFFECT_POISON_ID);
    assert_eq!(effects[1].duration, -20);
    assert_eq!(effects[2].effect.holder_id, VANILLA_MOB_EFFECT_HASTE_ID);
    assert_eq!(effects[2].duration, 160);

    let mut registries = RegistrySet::default();
    let mut contents = crate::RegistryContentState::new("minecraft:mob_effect");
    contents.entries = vec![
        crate::RegistryPacketEntry::stub("example:fake"),
        crate::RegistryPacketEntry::stub("minecraft:speed"),
    ];
    registries
        .contents
        .insert("minecraft:mob_effect".to_string(), contents);
    let overridden = decode_supported_component_value(
        VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID,
        "minecraft:suspicious_stew_effects",
        &value,
        Some(&registries),
    )
    .unwrap();
    assert!(matches!(
        overridden,
        Some(DialogDataComponentValueState::SuspiciousStewEffects { effects })
            if effects[0].effect.holder_id == VANILLA_MOB_EFFECT_SPEED_ID
    ));

    for (value, expected) in [
        (compound(Vec::new()), "is not a list"),
        (
            NbtSummaryValue::List(vec![NbtSummaryValue::String("speed".to_string())]),
            "[0] is not a compound",
        ),
        (
            NbtSummaryValue::List(vec![compound(Vec::new())]),
            "missing field id",
        ),
        (
            NbtSummaryValue::List(vec![compound(vec![string_entry(
                "id",
                "minecraft:missing",
            )])]),
            "missing minecraft:mob_effect resource",
        ),
    ] {
        let error = decode_supported_component_value(
            VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID,
            "minecraft:suspicious_stew_effects",
            &value,
            None,
        )
        .unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }
}

#[test]
fn suspicious_stew_fields_follow_compound_last_wins_semantics() {
    let value = NbtSummaryValue::List(vec![compound(vec![
        string_entry("id", "missing"),
        string_entry("id", "speed"),
        value_entry("duration", NbtSummaryValue::Int(10)),
        value_entry("duration", NbtSummaryValue::Int(20)),
    ])]);
    let decoded = decode_supported_component_value(
        VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID,
        "minecraft:suspicious_stew_effects",
        &value,
        None,
    )
    .unwrap();
    assert!(matches!(
        decoded,
        Some(DialogDataComponentValueState::SuspiciousStewEffects { effects })
            if effects[0].effect.resource_id == "minecraft:speed"
                && effects[0].duration == 20
    ));
}

#[test]
fn direct_trim_projection_retains_component_nbt() {
    let material_component = compound(vec![string_entry("translate", "trim.example.material")]);
    let pattern_component = compound(vec![string_entry("text", "Custom pattern")]);
    let mut patch = DataComponentPatchSummary::default();
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_TRIM_TYPE_ID,
        &DialogDataComponentValueState::Trim {
            material: DialogTrimMaterialHolderState::Direct {
                material: DialogTrimMaterialState {
                    asset_name: "opal".to_string(),
                    override_armor_assets: BTreeMap::new(),
                    description: DialogTextState {
                        text: "Custom material".to_string(),
                        runs: Vec::new(),
                        component: material_component.clone(),
                    },
                },
            },
            pattern: DialogTrimPatternHolderState::Direct {
                pattern: DialogTrimPatternState {
                    asset_id: "example:spiral".to_string(),
                    description: DialogTextState {
                        text: "Custom pattern".to_string(),
                        runs: Vec::new(),
                        component: pattern_component.clone(),
                    },
                    decal: true,
                },
            },
        },
        &mut patch,
    ));

    assert_eq!(
        patch
            .armor_trim_material_direct
            .as_deref()
            .and_then(|material| material.description_nbt.as_deref()),
        Some(&material_component)
    );
    assert_eq!(
        patch
            .armor_trim_pattern_direct
            .as_deref()
            .and_then(|pattern| pattern.description_nbt.as_deref()),
        Some(&pattern_component)
    );
}

#[test]
fn decodes_pot_decorations_with_builtin_item_holders_and_empty_sides() {
    let value = NbtSummaryValue::List(vec![
        NbtSummaryValue::String("brick".to_string()),
        NbtSummaryValue::String("minecraft:angler_pottery_sherd".to_string()),
        NbtSummaryValue::String("bow".to_string()),
    ]);
    let decoded = decode_supported_component_value(
        VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID,
        "minecraft:pot_decorations",
        &value,
        None,
    )
    .unwrap();
    let Some(DialogDataComponentValueState::PotDecorations { decorations }) = decoded else {
        panic!("expected pot-decorations component");
    };
    assert!(decorations.back.is_none());
    let left = decorations.left.as_ref().unwrap();
    assert_eq!(left.resource_id, "minecraft:angler_pottery_sherd");
    assert_eq!(
        left.holder_id,
        bbb_protocol::item_types::vanilla_item_type_for_resource_id(
            "minecraft:angler_pottery_sherd"
        )
        .unwrap()
        .id
    );
    assert_eq!(
        decorations.right.as_ref().unwrap().resource_id,
        "minecraft:bow"
    );
    assert!(decorations.front.is_none());

    let brick_id = bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:brick")
        .unwrap()
        .id;
    let mut patch = DataComponentPatchSummary::default();
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID,
        &DialogDataComponentValueState::PotDecorations {
            decorations: decorations.clone(),
        },
        &mut patch,
    ));
    assert_eq!(
        patch.pot_decorations_item_ids,
        vec![
            brick_id,
            left.holder_id,
            decorations.right.as_ref().unwrap().holder_id,
            brick_id,
        ]
    );

    let mut registries = RegistrySet::default();
    let mut contents = crate::RegistryContentState::new("minecraft:item");
    contents.entries = vec![
        crate::RegistryPacketEntry::stub("example:fake"),
        crate::RegistryPacketEntry::stub("minecraft:angler_pottery_sherd"),
    ];
    registries
        .contents
        .insert("minecraft:item".to_string(), contents);
    let overridden = decode_supported_component_value(
        VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID,
        "minecraft:pot_decorations",
        &value,
        Some(&registries),
    )
    .unwrap();
    assert!(matches!(
        overridden,
        Some(DialogDataComponentValueState::PotDecorations { decorations })
            if decorations.left.as_ref().unwrap().holder_id == left.holder_id
    ));

    let empty = decode_supported_component_value(
        VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID,
        "minecraft:pot_decorations",
        &NbtSummaryValue::List(Vec::new()),
        None,
    )
    .unwrap();
    assert!(matches!(
        &empty,
        Some(DialogDataComponentValueState::PotDecorations { decorations })
            if decorations.back.is_none()
                && decorations.left.is_none()
                && decorations.right.is_none()
                && decorations.front.is_none()
    ));
    let mut empty_patch = DataComponentPatchSummary::default();
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID,
        empty.as_ref().unwrap(),
        &mut empty_patch,
    ));
    assert_eq!(empty_patch.pot_decorations_item_ids, vec![brick_id; 4]);

    for (value, expected) in [
        (compound(Vec::new()), "is not a list"),
        (
            NbtSummaryValue::List(vec![NbtSummaryValue::Int(1)]),
            "holder is not a resource id",
        ),
        (
            NbtSummaryValue::List(vec![NbtSummaryValue::String(
                "minecraft:missing".to_string(),
            )]),
            "missing minecraft:item resource",
        ),
        (
            NbtSummaryValue::List(vec![NbtSummaryValue::String("brick".to_string()); 5]),
            "maximum is 4",
        ),
    ] {
        let error = decode_supported_component_value(
            VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID,
            "minecraft:pot_decorations",
            &value,
            None,
        )
        .unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }
}

#[test]
fn projects_holder_backed_component_values_into_protocol_summary() {
    let holder = |holder_id, resource_id: &str| DialogRegistryHolderReferenceState {
        holder_id,
        resource_id: resource_id.to_string(),
    };
    let direct_set = DialogRegistryHolderSetState {
        holder_set_kind: "direct".to_string(),
        tag: None,
        entries: vec![holder(3, "example:first"), holder(5, "example:second")],
    };
    let named_set = DialogRegistryHolderSetState {
        holder_set_kind: "named".to_string(),
        tag: Some("example:tag".to_string()),
        entries: vec![holder(7, "example:resolved")],
    };
    let mut patch = DataComponentPatchSummary::default();

    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID,
        &DialogDataComponentValueState::Repairable {
            items: direct_set.clone(),
        },
        &mut patch,
    ));
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
        &DialogDataComponentValueState::ProvidesBannerPatterns {
            patterns: named_set.clone(),
        },
        &mut patch,
    ));
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
        &DialogDataComponentValueState::BannerPatterns {
            layers: vec![
                DialogBannerPatternLayerState {
                    pattern: DialogBannerPatternHolderState::Named {
                        registry_id: 2,
                        resource_id: "example:named".to_string(),
                    },
                    color_id: VANILLA_DYE_COLOR_YELLOW_ID,
                    color: "yellow".to_string(),
                },
                DialogBannerPatternLayerState {
                    pattern: DialogBannerPatternHolderState::Direct {
                        pattern: DialogDirectBannerPatternState {
                            asset_id: "example:direct".to_string(),
                            translation_key: "block.example.banner.direct".to_string(),
                        },
                    },
                    color_id: VANILLA_DYE_COLOR_BLUE_ID,
                    color: "blue".to_string(),
                },
            ],
        },
        &mut patch,
    ));
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
        &DialogDataComponentValueState::JukeboxPlayable {
            song: holder(6, "example:song"),
        },
        &mut patch,
    ));
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID,
        &DialogDataComponentValueState::DamageResistant { types: named_set },
        &mut patch,
    ));
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID,
        &DialogDataComponentValueState::DamageType {
            damage_type: holder(8, "example:damage"),
        },
        &mut patch,
    ));
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID,
        &DialogDataComponentValueState::SuspiciousStewEffects {
            effects: vec![DialogSuspiciousStewEffectState {
                effect: holder(VANILLA_MOB_EFFECT_SLOWNESS_ID, "minecraft:slowness"),
                duration: 160,
            }],
        },
        &mut patch,
    ));
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID,
        &DialogDataComponentValueState::RegistryVariant {
            registry: "minecraft:wolf_variant".to_string(),
            holder: holder(9, "example:wolf"),
        },
        &mut patch,
    ));
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID,
        &DialogDataComponentValueState::PaintingVariant {
            variant: holder(10, "example:painting"),
        },
        &mut patch,
    ));

    assert_eq!(patch.repairable_item_registry_ids, vec![3, 5]);
    assert_eq!(
        patch.provides_banner_pattern_registry_ids,
        Vec::<i32>::new()
    );
    assert_eq!(
        patch.provides_banner_pattern_tag.as_deref(),
        Some("example:tag")
    );
    assert_eq!(patch.banner_pattern_layers.len(), 2);
    assert_eq!(patch.banner_pattern_layers[0].registry_id, Some(2));
    assert_eq!(
        patch.banner_pattern_layers[1].asset_id.as_deref(),
        Some("example:direct")
    );
    assert_eq!(patch.jukebox_song_id, Some(6));
    assert_eq!(patch.damage_resistant_type_registry_ids, Vec::<i32>::new());
    assert_eq!(
        patch.damage_resistant_type_tag.as_deref(),
        Some("example:tag")
    );
    assert_eq!(patch.damage_type_id, Some(8));
    assert_eq!(
        patch.suspicious_stew_effects,
        vec![SuspiciousStewEffectSummary {
            effect_id: VANILLA_MOB_EFFECT_SLOWNESS_ID,
            duration: 160,
        }]
    );
    assert_eq!(patch.wolf_variant_id, Some(9));
    assert_eq!(patch.painting_variant_id, Some(10));

    let mismatched_variant = DialogDataComponentValueState::RegistryVariant {
        registry: "minecraft:cat_variant".to_string(),
        holder: holder(1, "example:cat"),
    };
    assert!(!apply_item_patch_value(
        VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID,
        &mismatched_variant,
        &mut patch,
    ));
    assert!(!apply_item_patch_value(
        VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID,
        &DialogDataComponentValueState::Repairable {
            items: DialogRegistryHolderSetState {
                holder_set_kind: "named".to_string(),
                tag: None,
                entries: Vec::new(),
            },
        },
        &mut patch,
    ));
}

#[test]
fn projects_nonrecursive_item_behavior_values_into_protocol_summary() {
    let explosion = FireworkExplosionSummary {
        shape: FireworkExplosionShapeSummary::Creeper,
        colors: vec![0x11_22_33],
        fade_colors: vec![0x44_55_66],
        has_trail: true,
        has_twinkle: false,
    };
    let values = [
        (
            VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
            DialogDataComponentValueState::UseEffects {
                can_sprint: true,
                interact_vibrations: false,
                speed_multiplier_bits: 0.75f32.to_bits(),
            },
        ),
        (
            VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
            DialogDataComponentValueState::SwingAnimation {
                animation_type: SwingAnimationTypeSummary::Stab,
                duration: 9,
            },
        ),
        (
            VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID,
            DialogDataComponentValueState::BlockState {
                properties: vec![DialogStringMapEntryState {
                    key: "axis".to_string(),
                    value: "x".to_string(),
                }],
            },
        ),
        (
            VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
            DialogDataComponentValueState::Food {
                nutrition: 4,
                saturation_bits: 0.6f32.to_bits(),
                can_always_eat: true,
            },
        ),
        (
            VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
            DialogDataComponentValueState::UseCooldown {
                seconds_bits: 1.25f32.to_bits(),
                cooldown_group: Some("minecraft:ender_pearl".to_string()),
            },
        ),
        (
            VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID,
            DialogDataComponentValueState::Weapon {
                item_damage_per_attack: 3,
                disable_blocking_for_seconds_bits: 5.0f32.to_bits(),
            },
        ),
        (
            VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID,
            DialogDataComponentValueState::AttackRange {
                min_reach_bits: 1.0f32.to_bits(),
                max_reach_bits: 4.0f32.to_bits(),
                min_creative_reach_bits: 2.0f32.to_bits(),
                max_creative_reach_bits: 6.0f32.to_bits(),
                hitbox_margin_bits: 0.5f32.to_bits(),
                mob_factor_bits: 1.5f32.to_bits(),
            },
        ),
        (
            VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID,
            DialogDataComponentValueState::Enchantable { value: 12 },
        ),
        (
            VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
            DialogDataComponentValueState::FireworkExplosion {
                explosion: explosion.clone(),
            },
        ),
        (
            VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
            DialogDataComponentValueState::Fireworks {
                flight_duration: 2,
                explosions: vec![explosion.clone()],
            },
        ),
        (
            VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID,
            DialogDataComponentValueState::LodestoneTracker {
                target: Some(LodestoneTargetSummary {
                    dimension: "minecraft:the_nether".to_string(),
                    pos: BlockPos { x: 1, y: 2, z: 3 },
                }),
                tracked: false,
            },
        ),
    ];
    let mut patch = DataComponentPatchSummary::default();
    for (type_id, value) in values {
        assert!(apply_item_patch_value(type_id, &value, &mut patch));
    }

    assert_eq!(
        patch.use_effects,
        Some(UseEffectsSummary {
            can_sprint: true,
            interact_vibrations: false,
            speed_multiplier: 0.75,
        })
    );
    assert_eq!(
        patch.swing_animation,
        Some(SwingAnimationSummary {
            animation_type: SwingAnimationTypeSummary::Stab,
            duration: 9,
        })
    );
    assert_eq!(
        patch.block_state_properties.get("axis").map(String::as_str),
        Some("x")
    );
    assert_eq!(patch.food_nutrition, Some(4));
    assert_eq!(patch.food_saturation_bits, Some(0.6f32.to_bits()));
    assert!(patch.food_can_always_eat);
    assert_eq!(patch.use_cooldown_seconds_bits, Some(1.25f32.to_bits()));
    assert_eq!(patch.use_cooldown_ticks, Some(25));
    assert_eq!(
        patch.use_cooldown_group.as_deref(),
        Some("minecraft:ender_pearl")
    );
    assert_eq!(patch.weapon_item_damage_per_attack, Some(3));
    assert_eq!(
        patch.weapon_disable_blocking_seconds_bits,
        Some(5.0f32.to_bits())
    );
    let attack_range = patch.attack_range.unwrap();
    assert_eq!(attack_range.min_reach.to_bits(), 1.0f32.to_bits());
    assert_eq!(attack_range.mob_factor.to_bits(), 1.5f32.to_bits());
    assert_eq!(patch.enchantable_value, Some(12));
    assert_eq!(patch.firework_explosion_shape, Some(explosion.shape));
    assert_eq!(patch.fireworks_flight_duration, Some(2));
    assert_eq!(patch.fireworks_explosions_count, Some(1));
    assert_eq!(patch.fireworks_explosions, vec![explosion]);
    assert_eq!(
        patch.lodestone_target.as_ref().unwrap().pos,
        BlockPos { x: 1, y: 2, z: 3 }
    );
    assert_eq!(patch.lodestone_tracked, Some(false));
}

#[test]
fn projects_sound_bearing_weapon_values_into_protocol_summary() {
    let named_sound = DialogSoundEventHolderState::Named {
        registry_id: 42,
        resource_id: "example:named".to_string(),
    };
    let direct_sound = DialogSoundEventHolderState::Direct {
        sound_id: "example:direct".to_string(),
        fixed_range_bits: Some(12.5f32.to_bits()),
    };
    let condition = DialogKineticWeaponConditionState {
        max_duration_ticks: 20,
        min_speed_bits: 1.25f32.to_bits(),
        min_relative_speed_bits: 2.5f32.to_bits(),
    };
    let mut patch = DataComponentPatchSummary::default();

    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID,
        &DialogDataComponentValueState::BreakSound {
            sound: direct_sound.clone(),
        },
        &mut patch,
    ));
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
        &DialogDataComponentValueState::PiercingWeapon {
            weapon: DialogPiercingWeaponState {
                deals_knockback: false,
                dismounts: true,
                sound: Some(named_sound.clone()),
                hit_sound: Some(direct_sound.clone()),
            },
        },
        &mut patch,
    ));
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
        &DialogDataComponentValueState::KineticWeapon {
            weapon: DialogKineticWeaponState {
                contact_cooldown_ticks: 3,
                delay_ticks: 2,
                dismount_conditions: Some(condition.clone()),
                knockback_conditions: None,
                damage_conditions: Some(condition.clone()),
                forward_movement_bits: 0.75f32.to_bits(),
                damage_multiplier_bits: 1.5f32.to_bits(),
                sound: Some(direct_sound),
                hit_sound: Some(named_sound),
            },
        },
        &mut patch,
    ));

    assert_eq!(
        patch.break_sound,
        Some(SoundEventSummary {
            registry_id: None,
            sound_id: Some("example:direct".to_string()),
            fixed_range_bits: Some(12.5f32.to_bits()),
        })
    );
    assert_eq!(
        patch.piercing_weapon,
        Some(PiercingWeaponSummary {
            deals_knockback: false,
            dismounts: true,
            sound: Some(SoundEventSummary {
                registry_id: Some(42),
                sound_id: None,
                fixed_range_bits: None,
            }),
            hit_sound: Some(SoundEventSummary {
                registry_id: None,
                sound_id: Some("example:direct".to_string()),
                fixed_range_bits: Some(12.5f32.to_bits()),
            }),
        })
    );
    let kinetic = patch.kinetic_weapon.unwrap();
    assert_eq!(kinetic.contact_cooldown_ticks, 3);
    assert_eq!(kinetic.delay_ticks, 2);
    assert_eq!(
        kinetic.dismount_conditions,
        Some(KineticWeaponConditionSummary {
            max_duration_ticks: 20,
            min_speed_bits: 1.25f32.to_bits(),
            min_relative_speed_bits: 2.5f32.to_bits(),
        })
    );
    assert!(kinetic.knockback_conditions.is_none());
    assert_eq!(kinetic.damage_conditions, kinetic.dismount_conditions);
    assert_eq!(kinetic.forward_movement_bits, 0.75f32.to_bits());
    assert_eq!(kinetic.damage_multiplier_bits, 1.5f32.to_bits());
    assert_eq!(
        kinetic.sound.unwrap().sound_id.as_deref(),
        Some("example:direct")
    );
    assert_eq!(kinetic.hit_sound.unwrap().registry_id, Some(42));
}

fn decoded(
    type_id: i32,
    resource_id: &str,
    value: &NbtSummaryValue,
) -> Result<Option<DialogDataComponentValueState>, String> {
    decode_supported_component_value(type_id, resource_id, value, Some(&RegistrySet::default()))
}

fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(entries)
}

fn value_entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.to_string(),
        value,
    }
}

fn string_entry(name: &str, value: &str) -> NbtSummaryEntry {
    value_entry(name, NbtSummaryValue::String(value.to_string()))
}

fn float_vector(values: &[f32]) -> NbtSummaryValue {
    NbtSummaryValue::List(
        values
            .iter()
            .map(|value| NbtSummaryValue::Float(value.to_bits()))
            .collect(),
    )
}

fn firework_explosion(shape: &str) -> NbtSummaryValue {
    compound(vec![string_entry("shape", shape)])
}
