use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_CREATIVE_SLOT_LOCK_TYPE_ID, VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
    VANILLA_DATA_COMPONENT_DYE_TYPE_ID, VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID, VANILLA_DATA_COMPONENT_LORE_TYPE_ID,
};

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
    for (type_id, resource_id) in [
        (
            VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
            "minecraft:custom_name",
        ),
        (
            VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID,
            "minecraft:item_name",
        ),
        (VANILLA_DATA_COMPONENT_LORE_TYPE_ID, "minecraft:lore"),
    ] {
        assert_eq!(
            decoded(type_id, resource_id, &NbtSummaryValue::List(Vec::new())).unwrap(),
            None
        );
    }
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
        Some(DialogDataComponentValueState::Recipes {
            resource_ids: vec![
                "minecraft:stick".to_string(),
                "minecraft:custom".to_string(),
                "minecraft:stick".to_string(),
            ],
        })
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

    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID,
            "minecraft:container_loot",
            &compound(vec![
                string_entry("loot_table", "chests/simple_dungeon"),
                value_entry("seed", NbtSummaryValue::Double(12.75f64.to_bits())),
            ]),
        )
        .unwrap(),
        Some(DialogDataComponentValueState::ContainerLoot {
            loot_table: "minecraft:chests/simple_dungeon".to_string(),
            seed: 12,
        })
    );
    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID,
            "minecraft:container_loot",
            &compound(vec![string_entry("loot_table", ":empty")]),
        )
        .unwrap(),
        Some(DialogDataComponentValueState::ContainerLoot {
            loot_table: "minecraft:empty".to_string(),
            seed: 0,
        })
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

fn decoded(
    type_id: i32,
    resource_id: &str,
    value: &NbtSummaryValue,
) -> Result<Option<DialogDataComponentValueState>, String> {
    decode_supported_component_value(type_id, resource_id, value)
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
