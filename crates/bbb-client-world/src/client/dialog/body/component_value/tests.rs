use bbb_protocol::builtin_enum_component_values::{
    VANILLA_DYE_COLOR_BLUE_ID, VANILLA_DYE_COLOR_LIGHT_BLUE_ID, VANILLA_DYE_COLOR_RED_ID,
};
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
    VANILLA_DATA_COMPONENT_CREATIVE_SLOT_LOCK_TYPE_ID,
    VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID,
    VANILLA_DATA_COMPONENT_DYE_TYPE_ID, VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID, VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
    VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID,
    VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
    VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
    VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID, VANILLA_DATA_COMPONENT_TRIM_TYPE_ID,
    VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID,
};
use bbb_protocol::mob_effect_types::{
    VANILLA_MOB_EFFECT_HASTE_ID, VANILLA_MOB_EFFECT_POISON_ID, VANILLA_MOB_EFFECT_SPEED_ID,
};

use super::*;

#[test]
fn persistent_component_select_bridge_decodes_semantic_values_and_rejects_transients() {
    let scalar = crate::decode_persistent_component_select_value(
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
        &serde_json::json!(16),
        None,
    )
    .unwrap()
    .unwrap();
    assert_eq!(scalar.max_stack_size, Some(16));
    let object_list = crate::decode_persistent_component_select_value(
        VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID,
        &serde_json::json!({
            "floats": [0.25, 1.5],
            "flags": [true, false],
            "strings": ["first", "second"],
            "colors": [1193046]
        }),
        None,
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        object_list.custom_model_data_floats.to_vec(),
        vec![0.25, 1.5]
    );
    assert_eq!(object_list.custom_model_data_flags, [true, false]);
    assert_eq!(object_list.custom_model_data_strings, ["first", "second"]);
    assert_eq!(object_list.custom_model_data_colors, [1_193_046]);
    let float = crate::decode_persistent_component_select_value(
        VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
        &serde_json::json!(0.125),
        None,
    )
    .unwrap()
    .unwrap();
    assert_eq!(float.minimum_attack_charge_bits, Some(0.125_f32.to_bits()));
    let styled = crate::decode_persistent_component_select_value(
        VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
        &serde_json::json!({"text": "Styled", "bold": true}),
        None,
    )
    .unwrap()
    .unwrap();
    assert_eq!(styled.custom_name.as_deref(), Some("Styled"));
    assert_eq!(
        styled
            .custom_name_styled
            .as_deref()
            .and_then(|runs| runs.first())
            .and_then(|run| run.style.bold),
        Some(true)
    );

    let registry = "minecraft:damage_type";
    let mut registries = RegistrySet::default();
    let mut contents = crate::RegistryContentState::new(registry);
    contents.entries = vec![
        crate::RegistryPacketEntry::stub("minecraft:in_fire"),
        crate::RegistryPacketEntry::stub("example:custom"),
    ];
    registries.contents.insert(registry.to_string(), contents);
    registries.tags.insert(
        registry.to_string(),
        crate::RegistryTagState::from_materialized(
            registry.to_string(),
            std::collections::BTreeMap::from([("minecraft:is_fire".to_string(), vec![0])]),
        ),
    );
    let holder = crate::decode_persistent_component_select_value(
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID,
        &serde_json::json!("example:custom"),
        Some(&registries),
    )
    .unwrap()
    .unwrap();
    assert_eq!(holder.damage_type_id, Some(1));

    let holder_set = crate::decode_persistent_component_select_value(
        VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID,
        &serde_json::json!({"types": "#minecraft:is_fire"}),
        Some(&registries),
    )
    .unwrap()
    .unwrap();
    bbb_protocol::packets::validate_data_component_patch_summary(&holder_set).unwrap();
    assert_eq!(holder_set.damage_resistant_type_registry_ids, vec![0]);
    assert_eq!(
        holder_set.damage_resistant_type_tag.as_deref(),
        Some("minecraft:is_fire")
    );
    let saved = bbb_registry_codec::item_stack_summary_to_persistent_nbt_with_resolver(
        &bbb_protocol::packets::ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
            count: 1,
            component_patch: holder_set.into(),
        },
        &registries.frozen_access(),
    )
    .unwrap();
    assert_eq!(
        saved,
        bbb_protocol::parse_snbt_compound(
            r##"{id:"minecraft:stone",count:1,components:{"minecraft:damage_resistant":{types:"#minecraft:is_fire"}}}"##
        )
        .unwrap()
    );

    let error = crate::decode_persistent_component_select_value(
        VANILLA_DATA_COMPONENT_CREATIVE_SLOT_LOCK_TYPE_ID,
        &serde_json::json!({}),
        None,
    )
    .unwrap_err();
    assert!(error.contains("is not persistent"), "{error}");
}

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
            decoded(type_id, resource_id, &NbtSummaryValue::string(raw)).unwrap(),
            Some(DialogDataComponentValueState::ResourceId {
                resource_id: normalized.to_string(),
            })
        );
    }
    assert_eq!(
        decoded(
            VANILLA_DATA_COMPONENT_RARITY_TYPE_ID,
            "minecraft:rarity",
            &NbtSummaryValue::string("rare"),
        )
        .unwrap(),
        Some(DialogDataComponentValueState::Rarity {
            value: ItemRaritySummary::Rare,
        })
    );
}

#[test]
fn rejects_invalid_primitive_component_value_codecs() {
    for (type_id, resource_id, value) in [
        (
            VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
            "minecraft:max_stack_size",
            NbtSummaryValue::Int(0),
        ),
        (
            VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
            "minecraft:max_damage",
            NbtSummaryValue::Int(0),
        ),
        (
            VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
            "minecraft:damage",
            NbtSummaryValue::Int(-1),
        ),
        (
            VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID,
            "minecraft:repair_cost",
            NbtSummaryValue::Int(-1),
        ),
        (
            VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
            "minecraft:minimum_attack_charge",
            NbtSummaryValue::Float((-0.0f32).to_bits()),
        ),
        (
            VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
            "minecraft:potion_duration_scale",
            NbtSummaryValue::Float(f32::INFINITY.to_bits()),
        ),
        (
            VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID,
            "minecraft:enchantment_glint_override",
            NbtSummaryValue::string("true"),
        ),
        (
            VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID,
            "minecraft:unbreakable",
            NbtSummaryValue::Byte(1),
        ),
        (
            VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID,
            "minecraft:item_model",
            NbtSummaryValue::string("Invalid"),
        ),
        (
            VANILLA_DATA_COMPONENT_RARITY_TYPE_ID,
            "minecraft:rarity",
            NbtSummaryValue::string("legendary"),
        ),
    ] {
        let _ = decoded(type_id, resource_id, &value).unwrap_err();
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
                &NbtSummaryValue::string(serialized_name),
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
            NbtSummaryValue::string("scarlet"),
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
            NbtSummaryValue::string("missing"),
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
                    key: "axis".into(),
                    value: "x".into(),
                },
                DialogStringMapEntryState {
                    key: "waterlogged".into(),
                    value: "false".into(),
                },
            ],
        })
    );
}

#[test]
fn rejects_invalid_scalar_wrapper_and_flat_record_codecs() {
    for (type_id, resource_id, value) in [
        (
            VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID,
            "minecraft:ominous_bottle_amplifier",
            NbtSummaryValue::Int(5),
        ),
        (
            VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
            "minecraft:use_effects",
            NbtSummaryValue::Byte(1),
        ),
        (
            VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
            "minecraft:use_effects",
            compound(vec![value_entry(
                "speed_multiplier",
                NbtSummaryValue::Float((-0.0f32).to_bits()),
            )]),
        ),
        (
            VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
            "minecraft:swing_animation",
            compound(vec![string_entry("type", "slash")]),
        ),
        (
            VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
            "minecraft:swing_animation",
            compound(vec![value_entry("duration", NbtSummaryValue::Int(0))]),
        ),
        (
            VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID,
            "minecraft:block_state",
            compound(vec![value_entry("axis", NbtSummaryValue::Int(1))]),
        ),
    ] {
        let _ = decoded(type_id, resource_id, &value).unwrap_err();
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
    for (type_id, resource_id, value) in [
        (
            VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
            "minecraft:food",
            compound(vec![value_entry("nutrition", NbtSummaryValue::Int(1))]),
        ),
        (
            VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
            "minecraft:food",
            compound(vec![
                value_entry("nutrition", NbtSummaryValue::Int(-1)),
                value_entry("saturation", NbtSummaryValue::Float(0.0f32.to_bits())),
            ]),
        ),
        (
            VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
            "minecraft:use_cooldown",
            compound(vec![value_entry(
                "seconds",
                NbtSummaryValue::Float((-0.0f32).to_bits()),
            )]),
        ),
        (
            VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
            "minecraft:use_cooldown",
            compound(vec![
                value_entry("seconds", NbtSummaryValue::Float(1.0f32.to_bits())),
                string_entry("cooldown_group", "Invalid"),
            ]),
        ),
        (
            VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID,
            "minecraft:weapon",
            compound(vec![value_entry(
                "item_damage_per_attack",
                NbtSummaryValue::Int(-1),
            )]),
        ),
        (
            VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID,
            "minecraft:weapon",
            compound(vec![value_entry(
                "disable_blocking_for_seconds",
                NbtSummaryValue::Float((-0.0f32).to_bits()),
            )]),
        ),
        (
            VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID,
            "minecraft:attack_range",
            compound(vec![value_entry(
                "max_reach",
                NbtSummaryValue::Double(64.25f64.to_bits()),
            )]),
        ),
        (
            VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID,
            "minecraft:attack_range",
            compound(vec![value_entry(
                "mob_factor",
                NbtSummaryValue::Float(f32::NAN.to_bits()),
            )]),
        ),
        (
            VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID,
            "minecraft:enchantable",
            compound(vec![value_entry("value", NbtSummaryValue::Int(0))]),
        ),
    ] {
        let _ = decoded(type_id, resource_id, &value).unwrap_err();
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
                NbtSummaryValue::string("stick"),
                NbtSummaryValue::string(":custom"),
                NbtSummaryValue::string("stick"),
            ]),
        )
        .unwrap(),
        Some(recipes.clone())
    );
}

#[test]
fn rejects_invalid_resource_key_and_component_catalog_metadata() {
    for (type_id, resource_id, value) in [
        (
            VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID,
            "minecraft:recipes",
            compound(Vec::new()),
        ),
        (
            VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID,
            "minecraft:recipes",
            NbtSummaryValue::List(vec![NbtSummaryValue::Int(1)]),
        ),
        (
            VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID,
            "minecraft:recipes",
            NbtSummaryValue::List(vec![NbtSummaryValue::string("Invalid")]),
        ),
        (
            VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID,
            "minecraft:tooltip_display",
            compound(vec![value_entry(
                "hidden_components",
                NbtSummaryValue::string("custom_name"),
            )]),
        ),
        (
            VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID,
            "minecraft:tooltip_display",
            compound(vec![value_entry(
                "hidden_components",
                NbtSummaryValue::List(vec![NbtSummaryValue::string("missing")]),
            )]),
        ),
        (
            VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID,
            "minecraft:container_loot",
            compound(Vec::new()),
        ),
        (
            VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID,
            "minecraft:container_loot",
            compound(vec![string_entry("loot_table", "Invalid")]),
        ),
        (
            VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID,
            "minecraft:container_loot",
            compound(vec![
                string_entry("loot_table", "chests/simple_dungeon"),
                string_entry("seed", "twelve"),
            ]),
        ),
    ] {
        let _ = decoded(type_id, resource_id, &value).unwrap_err();
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
                        NbtSummaryValue::string("first"),
                        NbtSummaryValue::string("second"),
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
            strings: vec!["first".into(), "second".into()],
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
    for (type_id, resource_id, value) in [
        (
            VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID,
            "minecraft:dyed_color",
            float_vector(&[0.0, 1.0]),
        ),
        (
            VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID,
            "minecraft:dyed_color",
            NbtSummaryValue::List(vec![
                NbtSummaryValue::Float(0.0f32.to_bits()),
                NbtSummaryValue::string("green"),
                NbtSummaryValue::Float(1.0f32.to_bits()),
            ]),
        ),
        (
            VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID,
            "minecraft:custom_model_data",
            NbtSummaryValue::List(Vec::new()),
        ),
        (
            VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID,
            "minecraft:custom_model_data",
            compound(vec![value_entry("floats", NbtSummaryValue::Int(1))]),
        ),
        (
            VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID,
            "minecraft:custom_model_data",
            compound(vec![value_entry(
                "flags",
                NbtSummaryValue::List(vec![NbtSummaryValue::string("true")]),
            )]),
        ),
        (
            VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID,
            "minecraft:custom_model_data",
            compound(vec![value_entry(
                "strings",
                NbtSummaryValue::List(vec![NbtSummaryValue::Int(1)]),
            )]),
        ),
        (
            VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID,
            "minecraft:custom_model_data",
            compound(vec![value_entry(
                "colors",
                NbtSummaryValue::List(vec![float_vector(&[0.0, 0.5, 1.0, 0.5])]),
            )]),
        ),
    ] {
        let _ = decoded(type_id, resource_id, &value).unwrap_err();
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
    for (type_id, resource_id, value) in [
        (
            VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
            "minecraft:firework_explosion",
            NbtSummaryValue::List(Vec::new()),
        ),
        (
            VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
            "minecraft:firework_explosion",
            compound(Vec::new()),
        ),
        (
            VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
            "minecraft:firework_explosion",
            compound(vec![string_entry("shape", "sphere")]),
        ),
        (
            VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
            "minecraft:firework_explosion",
            compound(vec![value_entry("shape", NbtSummaryValue::Int(2))]),
        ),
        (
            VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
            "minecraft:firework_explosion",
            compound(vec![
                string_entry("shape", "star"),
                value_entry("colors", NbtSummaryValue::string("red")),
            ]),
        ),
        (
            VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
            "minecraft:firework_explosion",
            compound(vec![
                string_entry("shape", "star"),
                value_entry(
                    "fade_colors",
                    NbtSummaryValue::List(vec![NbtSummaryValue::string("blue")]),
                ),
            ]),
        ),
        (
            VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
            "minecraft:fireworks",
            compound(vec![string_entry("flight_duration", "one")]),
        ),
        (
            VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
            "minecraft:fireworks",
            compound(vec![value_entry("explosions", NbtSummaryValue::Int(1))]),
        ),
        (
            VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
            "minecraft:fireworks",
            compound(vec![value_entry(
                "explosions",
                NbtSummaryValue::List(vec![NbtSummaryValue::Int(1)]),
            )]),
        ),
        (
            VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
            "minecraft:fireworks",
            compound(vec![value_entry(
                "explosions",
                NbtSummaryValue::List(vec![firework_explosion("small_ball"); 257]),
            )]),
        ),
    ] {
        let _ = decoded(type_id, resource_id, &value).unwrap_err();
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
    for value in [
        NbtSummaryValue::List(Vec::new()),
        compound(vec![value_entry("target", NbtSummaryValue::Int(1))]),
        compound(vec![value_entry("target", compound(Vec::new()))]),
        compound(vec![value_entry(
            "target",
            compound(vec![
                value_entry("dimension", NbtSummaryValue::Int(0)),
                value_entry("pos", NbtSummaryValue::IntArray(vec![0, 0, 0])),
            ]),
        )]),
        compound(vec![value_entry(
            "target",
            compound(vec![
                string_entry("dimension", "Invalid"),
                value_entry("pos", NbtSummaryValue::IntArray(vec![0, 0, 0])),
            ]),
        )]),
        compound(vec![value_entry(
            "target",
            compound(vec![string_entry("dimension", "overworld")]),
        )]),
        compound(vec![value_entry(
            "target",
            compound(vec![
                string_entry("dimension", "overworld"),
                value_entry("pos", NbtSummaryValue::Int(0)),
            ]),
        )]),
        compound(vec![value_entry(
            "target",
            compound(vec![
                string_entry("dimension", "overworld"),
                value_entry("pos", NbtSummaryValue::IntArray(vec![0, 1])),
            ]),
        )]),
        compound(vec![value_entry(
            "target",
            compound(vec![
                string_entry("dimension", "overworld"),
                value_entry(
                    "pos",
                    NbtSummaryValue::List(vec![
                        NbtSummaryValue::Int(0),
                        NbtSummaryValue::string("one"),
                        NbtSummaryValue::Int(2),
                    ]),
                ),
            ]),
        )]),
        compound(vec![string_entry("tracked", "true")]),
    ] {
        assert!(decoded(
            VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID,
            "minecraft:lodestone_tracker",
            &value,
        )
        .is_err());
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
        &NbtSummaryValue::string("minecraft:flower"),
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

    for (value, context) in [
        (compound(Vec::new()), Some(&registries)),
        (
            NbtSummaryValue::List(vec![NbtSummaryValue::string("flower")]),
            Some(&registries),
        ),
        (
            NbtSummaryValue::List(vec![compound(vec![string_entry("color", "red")])]),
            Some(&registries),
        ),
        (
            NbtSummaryValue::List(vec![compound(vec![string_entry(
                "pattern",
                "minecraft:flower",
            )])]),
            Some(&registries),
        ),
        (
            NbtSummaryValue::List(vec![compound(vec![
                string_entry("pattern", "minecraft:flower"),
                string_entry("color", "chartreuse"),
            ])]),
            Some(&registries),
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
        ),
        (
            NbtSummaryValue::List(vec![compound(vec![
                value_entry("pattern", NbtSummaryValue::List(Vec::new())),
                string_entry("color", "red"),
            ])]),
            None,
        ),
        (value, None),
    ] {
        assert!(decode_supported_component_value(
            VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
            "minecraft:banner_patterns",
            &value,
            context,
        )
        .is_err());
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
        &NbtSummaryValue::string("minecraft:cat"),
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
        &NbtSummaryValue::string("minecraft:cat"),
        None,
    )
    .is_err());
    assert!(decode_supported_component_value(
        VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
        "minecraft:jukebox_playable",
        &compound(vec![string_entry("description", "not persistent")]),
        Some(&registries),
    )
    .is_err());
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
        &NbtSummaryValue::string("ponder_goat_horn"),
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
        &NbtSummaryValue::string("minecraft:quartz"),
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

    for (type_id, resource_id, value, context) in [
        (
            VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
            "minecraft:instrument",
            NbtSummaryValue::string("minecraft:ponder_goat_horn"),
            None,
        ),
        (
            VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
            "minecraft:instrument",
            NbtSummaryValue::string("minecraft:missing"),
            Some(&registries),
        ),
        (
            VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
            "minecraft:provides_trim_material",
            compound(vec![string_entry("asset_name", "quartz")]),
            Some(&registries),
        ),
    ] {
        assert!(decode_supported_component_value(type_id, resource_id, &value, context).is_err());
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

    for (value, context) in [
        (value.clone(), None),
        (
            compound(vec![string_entry("pattern", "minecraft:coast")]),
            Some(&registries),
        ),
        (
            compound(vec![string_entry("material", "minecraft:quartz")]),
            Some(&registries),
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
        ),
        (
            compound(vec![
                string_entry("material", "minecraft:quartz"),
                string_entry("pattern", "minecraft:missing"),
            ]),
            Some(&registries),
        ),
    ] {
        assert!(decode_supported_component_value(
            VANILLA_DATA_COMPONENT_TRIM_TYPE_ID,
            "minecraft:trim",
            &value,
            context,
        )
        .is_err());
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
            &NbtSummaryValue::string("example:selected"),
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

    for (value, context) in [
        (NbtSummaryValue::string("example:selected"), None),
        (
            NbtSummaryValue::string("example:missing"),
            Some(&registries),
        ),
        (
            compound(vec![string_entry("texture", "example:wolf")]),
            Some(&registries),
        ),
    ] {
        assert!(decode_supported_component_value(
            VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID,
            "minecraft:wolf/variant",
            &value,
            context,
        )
        .is_err());
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
        &NbtSummaryValue::string("kebab"),
        Some(&registries),
    )
    .unwrap();
    let Some(DialogDataComponentValueState::PaintingVariant { variant }) = decoded else {
        panic!("expected painting-variant component");
    };
    assert_eq!(variant.holder_id, 1);
    assert_eq!(variant.resource_id, "minecraft:kebab");

    for (value, registries) in [
        (NbtSummaryValue::string("minecraft:kebab"), None),
        (
            NbtSummaryValue::string("minecraft:missing"),
            Some(&registries),
        ),
        (
            compound(vec![value_entry("width", NbtSummaryValue::Int(1))]),
            Some(&registries),
        ),
    ] {
        assert!(decode_supported_component_value(
            VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID,
            "minecraft:painting/variant",
            &value,
            registries,
        )
        .is_err());
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
        crate::RegistryTagState::from_materialized(
            registry.to_string(),
            std::collections::BTreeMap::from([("minecraft:is_fire".to_string(), vec![0])]),
        ),
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
    .is_err());
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

    for value in [
        compound(Vec::new()),
        NbtSummaryValue::List(vec![NbtSummaryValue::string("speed")]),
        NbtSummaryValue::List(vec![compound(Vec::new())]),
        NbtSummaryValue::List(vec![compound(vec![string_entry(
            "id",
            "minecraft:missing",
        )])]),
    ] {
        assert!(decode_supported_component_value(
            VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID,
            "minecraft:suspicious_stew_effects",
            &value,
            None,
        )
        .is_err());
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
fn decodes_pot_decorations_with_builtin_item_holders_and_empty_sides() {
    let value = NbtSummaryValue::List(vec![
        NbtSummaryValue::string("brick"),
        NbtSummaryValue::string("minecraft:angler_pottery_sherd"),
        NbtSummaryValue::string("bow"),
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
        name: name.into(),
        value,
    }
}

fn string_entry(name: &str, value: &str) -> NbtSummaryEntry {
    value_entry(name, NbtSummaryValue::string(value))
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
