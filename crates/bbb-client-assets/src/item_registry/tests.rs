use super::*;
use bbb_protocol::data_component_types as component_id;

#[test]
fn generated_common_item_components_apply_to_full_vanilla_item_catalog() {
    let catalog = ItemRegistryCatalog::from_generated_source_catalog().unwrap();
    for item in bbb_protocol::item_types::VANILLA_ITEM_TYPES {
        let actual = catalog
            .default_component_codec_values(item.resource_id)
            .unwrap_or_else(|| panic!("missing {} default components", item.resource_id));
        assert!(
            component_id::VANILLA_COMMON_ITEM_COMPONENT_TYPE_IDS
                .iter()
                .all(|type_id| actual.contains_key(type_id)),
            "{} omitted a generated common item component",
            item.resource_id
        );
    }
}

#[test]
fn extracts_complete_component_membership_and_scalar_values_by_protocol_id() {
    let catalog = ItemRegistryCatalog::from_generated_source_catalog().unwrap();
    let values = catalog.default_component_codec_values_by_protocol_id();
    let item = |resource_id| catalog.protocol_id(resource_id).unwrap();
    let component = |resource_id, type_id| values[&item(resource_id)].get(&type_id);
    let dye_type = component_id::VANILLA_DATA_COMPONENT_DYE_TYPE_ID;
    let scale_type = component_id::VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID;
    let cooldown_type = component_id::VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID;

    assert_eq!(values.len(), 1506);
    for (type_id, count) in [(dye_type, 16), (scale_type, 2), (cooldown_type, 3)] {
        assert_eq!(
            values
                .values()
                .filter(|values| values.contains_key(&type_id))
                .count(),
            count
        );
    }
    assert!(values[&item("minecraft:white_dye")].contains_key(&dye_type));
    assert!(values[&item("minecraft:diamond")]
        .contains_key(&component_id::VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID));
    assert!(values[&item("minecraft:tipped_arrow")].contains_key(&scale_type));
    assert_eq!(
        component("minecraft:white_dye", dye_type),
        Some(&serde_json::json!("white"))
    );
    assert_eq!(
        component("minecraft:red_dye", dye_type),
        Some(&serde_json::json!("red"))
    );
    assert_eq!(
        component("minecraft:tipped_arrow", scale_type),
        Some(&serde_json::json!(0.125))
    );
    assert_eq!(
        component("minecraft:lingering_potion", scale_type),
        Some(&serde_json::json!(0.25))
    );
    assert_eq!(component("minecraft:splash_potion", scale_type), None);
    for (resource_id, seconds) in [
        ("minecraft:ender_pearl", 1.0),
        ("minecraft:wind_charge", 0.5),
        ("minecraft:chorus_fruit", 1.0),
    ] {
        assert_eq!(
            component(resource_id, cooldown_type).unwrap()["seconds"],
            serde_json::json!(seconds)
        );
    }
    assert_eq!(component("minecraft:snowball", cooldown_type), None);
}

#[test]
fn generated_prototype_codec_values_match_generated_facts() {
    let catalog = ItemRegistryCatalog::from_generated_source_catalog().unwrap();
    let values = catalog.default_component_codec_values_by_protocol_id();
    let prototypes = bbb_protocol::vanilla_item_prototype_component_catalog().unwrap();

    assert_eq!(values.len(), 1506);
    assert_eq!(values.values().map(BTreeMap::len).sum::<usize>(), 18_981);
    for item in bbb_protocol::item_types::VANILLA_ITEM_TYPES {
        assert_eq!(
            &values[&item.id],
            prototypes.component_codec_values(item.id).unwrap(),
            "{} prototype values diverged from generated facts",
            item.resource_id
        );
    }

    let item_values = |resource_id| catalog.default_component_codec_values(resource_id).unwrap();
    let damage_type = component_id::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID;
    assert_eq!(
        item_values("minecraft:wooden_spear")[&damage_type].as_str(),
        Some("minecraft:spear")
    );
    assert!(!item_values("minecraft:diamond_sword").contains_key(&damage_type));

    assert_eq!(
        item_values("minecraft:stick")[&component_id::VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID],
        serde_json::json!({"translate": "item.minecraft.stick"})
    );
    assert_eq!(
        item_values("minecraft:wooden_spear")
            [&component_id::VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID],
        serde_json::json!({"type": "stab", "duration": 13})
    );
    assert_eq!(
        item_values("minecraft:diamond_chestplate")
            [&component_id::VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID],
        serde_json::json!({"items": "#minecraft:repairs_diamond_armor"})
    );
    assert_eq!(
        item_values("minecraft:creeper_spawn_egg")
            [&component_id::VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID],
        serde_json::json!({"id": "minecraft:creeper"})
    );
}

#[test]
fn minimum_attack_charge_defaults_follow_vanilla_spear_properties() {
    let catalog = ItemRegistryCatalog::from_generated_source_catalog().unwrap();
    for resource_id in [
        "minecraft:wooden_spear",
        "minecraft:stone_spear",
        "minecraft:copper_spear",
        "minecraft:iron_spear",
        "minecraft:golden_spear",
        "minecraft:diamond_spear",
        "minecraft:netherite_spear",
    ] {
        assert_eq!(
            catalog
                .default_component_codec_values(resource_id)
                .unwrap()
                .get(&component_id::VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID),
            Some(&serde_json::json!(1.0)),
            "{resource_id}"
        );
    }
    assert_eq!(
        catalog
            .default_component_codec_values("minecraft:diamond_sword")
            .unwrap()
            .get(&component_id::VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID),
        None
    );
}

#[test]
fn swing_animation_defaults_preserve_generated_type_and_duration() {
    let catalog = ItemRegistryCatalog::from_generated_source_catalog().unwrap();
    for (resource_id, duration) in [
        ("minecraft:wooden_spear", 13),
        ("minecraft:stone_spear", 15),
        ("minecraft:copper_spear", 17),
        ("minecraft:iron_spear", 19),
        ("minecraft:golden_spear", 19),
        ("minecraft:diamond_spear", 21),
        ("minecraft:netherite_spear", 23),
    ] {
        assert_eq!(
            catalog
                .default_component_codec_values(resource_id)
                .unwrap()
                .get(&component_id::VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID),
            Some(&serde_json::json!({"type": "stab", "duration": duration})),
            "{resource_id}"
        );
    }
    assert_eq!(
        catalog
            .default_component_codec_values("minecraft:diamond_sword")
            .unwrap()
            .get(&component_id::VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID),
        Some(&serde_json::json!({}))
    );
}

#[test]
fn loads_checked_in_vanilla_item_registry() {
    let catalog = ItemRegistryCatalog::from_generated_source_catalog().unwrap();

    assert_eq!(catalog.len(), 1506);
    assert_eq!(catalog.resource_id(0), Some("minecraft:air"));
    assert_eq!(catalog.resource_id(1), Some("minecraft:stone"));
    assert_eq!(catalog.resource_id(1505), Some("minecraft:ominous_bottle"));
    assert_eq!(catalog.protocol_id("ominous_bottle"), Some(1505));
    assert_eq!(catalog.protocol_id("minecraft:short_dry_grass"), Some(209));
    assert_eq!(catalog.protocol_id("minecraft:dry_short_grass"), None);
    let max_damage_type = component_id::VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID;
    let max_stack_type = component_id::VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID;
    assert_eq!(
        catalog.default_component_codec_values("elytra").unwrap()[&max_damage_type],
        serde_json::json!(432)
    );
    assert_eq!(
        catalog
            .default_component_codec_values("stone")
            .unwrap()
            .get(&max_damage_type),
        None
    );
    for (resource_id, stack_size) in [
        ("stone", 64),
        ("ender_pearl", 16),
        ("diamond_sword", 1),
        ("diamond_shovel", 1),
    ] {
        assert_eq!(
            catalog.default_component_codec_values(resource_id).unwrap()[&max_stack_type],
            serde_json::json!(stack_size)
        );
    }
    let kinetic_type = component_id::VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID;
    for (resource_id, expected) in [
        ("minecraft:wooden_spear", (15, 100, 200, 300)),
        ("minecraft:stone_spear", (14, 90, 180, 275)),
        ("minecraft:copper_spear", (13, 80, 165, 250)),
        ("minecraft:iron_spear", (12, 50, 135, 225)),
        ("minecraft:golden_spear", (14, 70, 170, 275)),
        ("minecraft:diamond_spear", (10, 60, 130, 200)),
        ("minecraft:netherite_spear", (8, 50, 110, 175)),
    ] {
        let defaults = &catalog.default_component_codec_values(resource_id).unwrap()[&kinetic_type];
        assert_eq!(
            (
                defaults["delay_ticks"].as_i64().unwrap(),
                defaults["dismount_conditions"]["max_duration_ticks"]
                    .as_i64()
                    .unwrap(),
                defaults["knockback_conditions"]["max_duration_ticks"]
                    .as_i64()
                    .unwrap(),
                defaults["damage_conditions"]["max_duration_ticks"]
                    .as_i64()
                    .unwrap(),
            ),
            expected,
            "{resource_id}"
        );
        assert_eq!(
            (defaults["forward_movement"].as_f64().unwrap() as f32).to_bits(),
            0.38f32.to_bits()
        );
    }
    assert_eq!(
        catalog
            .default_component_codec_values("minecraft:diamond_sword")
            .unwrap()
            .get(&kinetic_type),
        None
    );
    let spear = catalog
        .default_component_codec_values("wooden_spear")
        .unwrap();
    assert_eq!(
        spear[&component_id::VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID],
        serde_json::json!({
            "min_reach": 2.0, "max_reach": 4.5, "min_creative_reach": 2.0,
            "max_creative_reach": 6.5, "hitbox_margin": 0.125, "mob_factor": 0.5
        })
    );
    assert_eq!(
        spear[&component_id::VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID],
        serde_json::json!(1.0)
    );
    assert_eq!(
        spear[&component_id::VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID],
        serde_json::json!({
            "can_sprint": true, "interact_vibrations": false, "speed_multiplier": 1.0
        })
    );
    let tool_type = component_id::VANILLA_DATA_COMPONENT_TOOL_TYPE_ID;
    assert_eq!(
        catalog
            .default_component_codec_values("diamond_pickaxe")
            .unwrap()[&tool_type],
        serde_json::json!({"rules": [
            {"blocks": "#minecraft:incorrect_for_diamond_tool", "correct_for_drops": false},
            {"blocks": "#minecraft:mineable/pickaxe", "speed": 8.0, "correct_for_drops": true}
        ]})
    );
    let sword = &catalog
        .default_component_codec_values("diamond_sword")
        .unwrap()[&tool_type];
    assert_eq!(
        sword,
        &serde_json::json!({
            "can_destroy_blocks_in_creative": false,
            "damage_per_block": 2,
            "rules": [
                {"blocks": "minecraft:cobweb", "speed": 15.0, "correct_for_drops": true},
                {"blocks": "#minecraft:sword_instantly_mines", "speed": 3.4028235e38},
                {"blocks": "#minecraft:sword_efficient", "speed": 1.5}
            ]
        })
    );
    assert_eq!(
        (sword["rules"][1]["speed"].as_f64().unwrap() as f32).to_bits(),
        f32::MAX.to_bits()
    );
    assert_eq!(
        catalog.default_component_codec_values("shears").unwrap()[&tool_type],
        serde_json::json!({"rules": [
            {"blocks": "minecraft:cobweb", "speed": 15.0, "correct_for_drops": true},
            {"blocks": "#minecraft:leaves", "speed": 15.0},
            {"blocks": "#minecraft:wool", "speed": 5.0},
            {"blocks": ["minecraft:vine", "minecraft:glow_lichen"], "speed": 2.0}
        ]})
    );
    for item in ["mace", "trident"] {
        assert_eq!(
            catalog.default_component_codec_values(item).unwrap()[&tool_type],
            serde_json::json!({
                "rules": [], "damage_per_block": 2, "can_destroy_blocks_in_creative": false
            })
        );
    }
    let default_piercing_weapon_ids: Vec<_> = catalog
        .default_component_codec_values_by_protocol_id()
        .into_iter()
        .filter(|(_, values)| {
            values.contains_key(&component_id::VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID)
        })
        .filter_map(|(protocol_id, _)| catalog.resource_id(protocol_id))
        .collect();
    assert_eq!(
        default_piercing_weapon_ids,
        vec![
            "minecraft:wooden_spear",
            "minecraft:stone_spear",
            "minecraft:copper_spear",
            "minecraft:iron_spear",
            "minecraft:golden_spear",
            "minecraft:diamond_spear",
            "minecraft:netherite_spear",
        ]
    );
    let equippable_type = component_id::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID;
    for (resource_id, slot) in [
        ("minecraft:diamond_boots", "feet"),
        ("minecraft:elytra", "chest"),
        ("minecraft:shield", "offhand"),
        ("minecraft:red_harness", "body"),
    ] {
        assert_eq!(
            catalog.default_component_codec_values(resource_id).unwrap()[&equippable_type]["slot"],
            serde_json::json!(slot)
        );
    }
    assert!(!catalog
        .default_component_codec_values("minecraft:stone")
        .unwrap()
        .contains_key(&equippable_type));

    let components = |resource_id| catalog.default_component_codec_values(resource_id).unwrap();
    assert_eq!(components("minecraft:stick").len(), 12);
    assert!(components("minecraft:elytra")
        .contains_key(&component_id::VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID));
    assert!(components("minecraft:apple")
        .contains_key(&component_id::VANILLA_DATA_COMPONENT_FOOD_TYPE_ID));
    assert!(components("minecraft:iron_sword")
        .contains_key(&component_id::VANILLA_DATA_COMPONENT_TOOL_TYPE_ID));
    assert!(components("minecraft:wooden_spear")
        .contains_key(&component_id::VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID));
    assert!(components("minecraft:creeper_spawn_egg")
        .contains_key(&component_id::VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID));

    let food_type = component_id::VANILLA_DATA_COMPONENT_FOOD_TYPE_ID;
    for (resource_id, nutrition, saturation, can_always_eat) in [
        ("apple", 4, 2.4, false),
        ("chorus_fruit", 4, 2.4, true),
        ("salmon_bucket", 2, 0.4, false),
    ] {
        let food = &catalog.default_component_codec_values(resource_id).unwrap()[&food_type];
        assert_eq!(food["nutrition"], serde_json::json!(nutrition));
        assert_eq!(food["saturation"], serde_json::json!(saturation));
        assert_eq!(
            food.get("can_always_eat")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
            can_always_eat
        );
    }
    let consumable_type = component_id::VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID;
    for (resource_id, expected) in [
        ("apple", serde_json::json!({})),
        ("dried_kelp", serde_json::json!({"consume_seconds": 0.8})),
        (
            "milk_bucket",
            serde_json::json!({
                "animation": "drink",
                "has_consume_particles": false,
                "on_consume_effects": [{"type": "minecraft:clear_all_effects"}],
                "sound": "minecraft:entity.generic.drink"
            }),
        ),
    ] {
        assert_eq!(
            catalog.default_component_codec_values(resource_id).unwrap()[&consumable_type],
            expected
        );
    }
    assert_eq!(
        catalog.crafting_remainder("minecraft:water_bucket"),
        Some("minecraft:bucket")
    );
    assert_eq!(
        catalog.crafting_remainder("minecraft:honey_bottle"),
        Some("minecraft:glass_bottle")
    );
    for (resource_id, asset_id) in [
        ("minecraft:diamond_helmet", "diamond"),
        ("minecraft:elytra", "elytra"),
        ("minecraft:wolf_armor", "armadillo_scute"),
        ("minecraft:iron_horse_armor", "iron"),
        ("minecraft:iron_nautilus_armor", "iron"),
        ("minecraft:saddle", "saddle"),
        ("minecraft:white_carpet", "white_carpet"),
        ("minecraft:white_harness", "white_harness"),
    ] {
        assert_eq!(
            catalog.default_component_codec_values(resource_id).unwrap()[&equippable_type]
                ["asset_id"],
            serde_json::json!(format!("minecraft:{asset_id}")),
            "{resource_id}"
        );
    }
    let attributes_type = component_id::VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID;
    for (resource_id, damage, speed) in [
        ("iron_sword", 5.0, f64::from(-2.4_f32)),
        ("iron_shovel", 3.5, -3.0),
        ("iron_axe", 8.0, f64::from(-3.1_f32)),
        ("mace", 5.0, f64::from(-3.4_f32)),
        ("trident", 8.0, f64::from(-2.9_f32)),
    ] {
        assert_eq!(
            catalog.default_component_codec_values(resource_id).unwrap()[&attributes_type],
            serde_json::json!([
                {
                    "type": "minecraft:attack_damage",
                    "id": "minecraft:base_attack_damage",
                    "amount": damage,
                    "operation": "add_value",
                    "slot": "mainhand"
                },
                {
                    "type": "minecraft:attack_speed",
                    "id": "minecraft:base_attack_speed",
                    "amount": speed,
                    "operation": "add_value",
                    "slot": "mainhand"
                }
            ]),
            "{resource_id}"
        );
    }
    assert_eq!(
        catalog
            .default_component_codec_values("carved_pumpkin")
            .unwrap()[&attributes_type],
        serde_json::json!([{
            "type": "minecraft:waypoint_transmit_range",
            "id": "minecraft:waypoint_transmit_range_hide",
            "amount": -1.0,
            "operation": "add_multiplied_total",
            "slot": "head",
            "display": {"type": "hidden"}
        }])
    );
}

#[test]
fn checked_in_item_catalog_matches_generated_protocol_catalog() {
    let catalog = ItemRegistryCatalog::from_generated_source_catalog().unwrap();
    assert_eq!(
        catalog.len(),
        bbb_protocol::item_types::VANILLA_ITEM_TYPES.len()
    );
    for (id, (resource_id, generated)) in catalog
        .resource_ids()
        .iter()
        .zip(bbb_protocol::item_types::VANILLA_ITEM_TYPES)
        .enumerate()
    {
        assert_eq!(generated.id, id as i32);
        assert_eq!(generated.resource_id, resource_id);
    }
    assert_eq!(
        catalog.primary_block_resource_ids.len(),
        catalog.block_items.len(),
        "every generated BlockItem must have one primary block"
    );
    for (block, item) in &catalog.block_item_resource_ids {
        assert!(
            catalog.protocol_id(item).is_some(),
            "unknown mapped item {item}"
        );
        assert!(
            catalog.block_items.contains(item),
            "mapped item {item} for {block} is not classified as a BlockItem"
        );
    }
    assert_eq!(
        catalog.item_resource_id_for_block("minecraft:stone"),
        Some("minecraft:stone")
    );
    assert_eq!(
        catalog.item_resource_id_for_block("minecraft:redstone_wire"),
        Some("minecraft:redstone")
    );
    assert_eq!(
        catalog.item_resource_id_for_block("minecraft:redstone_wall_torch"),
        Some("minecraft:redstone_torch")
    );
    assert_eq!(
        catalog.primary_block_resource_id_for_item("minecraft:redstone_torch"),
        Some("minecraft:redstone_torch")
    );
    assert_eq!(
        catalog.primary_block_resource_id_for_item("minecraft:redstone"),
        Some("minecraft:redstone_wire")
    );
    assert_eq!(
        catalog.item_resource_id_for_block("minecraft:weathered_copper_bars"),
        Some("minecraft:weathered_copper_bars")
    );
    assert_eq!(catalog.item_resource_id_for_block("minecraft:water"), None);
}

#[test]
fn ordinary_catalog_load_does_not_require_local_vanilla_sources() {
    let missing = std::path::PathBuf::from("/bbb-test/missing-vanilla-source-root");
    let roots = PackRoots::from_paths(
        missing.clone(),
        missing.join("sources/26.1"),
        missing.join("sources/26.1/assets/minecraft"),
    );

    let catalog = ItemRegistryCatalog::load(&roots).unwrap();

    assert_eq!(catalog.len(), 1506);
    assert_eq!(catalog.resource_id(0), Some("minecraft:air"));
}
