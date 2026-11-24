use super::*;

#[test]
fn native_item_runtime_resolves_component_condition_predicates() {
    let h = ComponentConditionHarness::new();
    // `ComponentMatches` with a component-type discriminator uses
    // `AnyValue.matches`, so vanilla common default components are present.
    h.assert_cases(
        0,
        "component_condition_rarity",
        [
            (DataComponentPatchSummary::default(), true),
            (patch_removed(&[12]), false),
        ],
    );

    // Non-default components are present only when the stack patch carries
    // that component, regardless of the component's boolean payload.
    h.assert_cases(
        1,
        "component_condition_glint",
        [
            (DataComponentPatchSummary::default(), false),
            (
                patch_with(&[21], |p| p.enchantment_glint_override = Some(false)),
                true,
            ),
            (
                patch_with(&[21], |p| {
                    p.enchantment_glint_override = Some(false);
                    p.removed_type_ids = vec![21];
                }),
                false,
            ),
        ],
    );

    // Vanilla `DamagePredicate.matches` requires the `minecraft:damage`
    // component and matches both damage and durability (`max_damage -
    // damage`) with `MinMaxBounds.Ints`.
    h.assert_cases(
        2,
        "component_condition_damage",
        [
            (DataComponentPatchSummary::default(), false),
            (patch_with(&[3], |p| p.damage = Some(3)), true),
            (patch_with(&[3], |p| p.damage = Some(4)), false),
            (
                patch_with(&[3], |p| {
                    p.removed_type_ids = vec![3];
                    p.damage = Some(3);
                }),
                false,
            ),
            (
                patch_with(&[3], |p| {
                    p.removed_type_ids = vec![2];
                    p.damage = Some(3);
                }),
                false,
            ),
        ],
    );

    for (item_id, component_id, model_id) in [
        (3, 50, "component_condition_bundle_contents"),
        (4, 56, "component_condition_trim"),
        (5, 68, "component_condition_firework_explosion"),
        (6, 69, "component_condition_fireworks"),
        (7, 64, "component_condition_jukebox_playable"),
        (8, 75, "component_condition_container"),
    ] {
        assert_eq!(
            h.selected(item_id, DataComponentPatchSummary::default()),
            h.uv(&format!("{model_id}_absent"))
        );
        assert_eq!(
            h.selected(
                item_id,
                patch_with(&[component_id], |p| p.firework_explosion = (component_id == bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID).then(Default::default))
            ),
            h.uv(&format!("{model_id}_present"))
        );
        assert_eq!(
            h.selected(
                item_id,
                patch_with(&[component_id], |p| { p.firework_explosion = (component_id == bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID).then(Default::default); p.removed_type_ids = vec![component_id]; })
            ),
            h.uv(&format!("{model_id}_absent"))
        );
    }
    for (item_resource_id, component_id, model_id) in [
        (
            "minecraft:component_condition_use_remainder",
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
            "component_condition_use_remainder",
        ),
        (
            "minecraft:component_condition_charged_projectiles",
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID,
            "component_condition_charged_projectiles",
        ),
    ] {
        let item_id = h
            .runtime
            .registry
            .as_ref()
            .and_then(|registry| registry.protocol_id(item_resource_id))
            .unwrap();
        assert_eq!(
            h.selected(item_id, DataComponentPatchSummary::default()),
            h.uv(&format!("{model_id}_absent"))
        );
        assert_eq!(
            h.selected(item_id, patch_with(&[component_id], |_| {})),
            h.uv(&format!("{model_id}_present"))
        );
        assert_eq!(
            h.selected(
                item_id,
                patch_with(&[component_id], |p| p.removed_type_ids = vec![component_id])
            ),
            h.uv(&format!("{model_id}_absent"))
        );
    }

    h.assert_cases(
        9,
        "component_condition_bundle_contents_constrained",
        [
            (DataComponentPatchSummary::default(), false),
            (
                bundle_patch(vec![
                    plain_template_stack(
                        bbb_protocol::item_types::VANILLA_ITEM_STONE_ID,
                        1
                    );
                    1
                ]),
                true,
            ),
            (
                bundle_patch(vec![
                    plain_template_stack(
                        bbb_protocol::item_types::VANILLA_ITEM_STONE_ID,
                        1
                    );
                    2
                ]),
                false,
            ),
            (patch_with(&[50], |_| {}), false),
            (
                patch_with(&[50], |p| {
                    p.removed_type_ids = vec![50];
                    p.bundle_contents_items = vec![
                        plain_template_stack(
                            bbb_protocol::item_types::VANILLA_ITEM_STONE_ID,
                            1
                        );
                        1
                    ];
                }),
                false,
            ),
        ],
    );
    h.assert_cases(
        21,
        "component_condition_bundle_contains",
        [
            (bundle_patch(vec![plain_template_stack(0, 3)]), true),
            (bundle_patch(vec![plain_template_stack(0, 1)]), false),
            (
                patch_with(&[50], |p| {
                    p.removed_type_ids = vec![50];
                    p.bundle_contents_items = vec![plain_template_stack(0, 3)];
                }),
                false,
            ),
        ],
    );
    h.assert_cases(
        22,
        "component_condition_bundle_count",
        [
            (
                bundle_patch(vec![
                    plain_template_stack(0, 1),
                    plain_template_stack(1, 1),
                    plain_template_stack(2, 1),
                ]),
                true,
            ),
            (
                bundle_patch(vec![plain_template_stack(0, 1), plain_template_stack(2, 1)]),
                false,
            ),
        ],
    );
    h.assert_cases(
        23,
        "component_condition_container_contains",
        [
            (
                container_patch(vec![plain_template_stack(1, 4)], Some(1)),
                true,
            ),
            (
                container_patch(vec![plain_template_stack(1, 3)], Some(1)),
                false,
            ),
            (
                patch_with(&[75], |p| {
                    p.removed_type_ids = vec![75];
                    p.container = container_from_items(vec![plain_template_stack(1, 4)], Some(1));
                }),
                false,
            ),
        ],
    );
    h.assert_cases(
        24,
        "component_condition_container_count",
        [
            (
                container_patch(
                    vec![plain_template_stack(0, 1), plain_template_stack(1, 1)],
                    Some(2),
                ),
                true,
            ),
            (
                container_patch(vec![plain_template_stack(0, 1)], Some(1)),
                false,
            ),
        ],
    );
    h.assert_cases(
        28,
        "component_condition_bundle_tag_contains",
        [
            (bundle_patch(vec![plain_template_stack(0, 2)]), true),
            (bundle_patch(vec![plain_template_stack(2, 2)]), false),
            (
                patch_with(&[50], |p| {
                    p.removed_type_ids = vec![50];
                    p.bundle_contents_items = vec![plain_template_stack(0, 2)];
                }),
                false,
            ),
        ],
    );
    h.assert_cases(
        29,
        "component_condition_container_tag_count",
        [
            (
                container_patch(
                    vec![
                        plain_template_stack(0, 1),
                        plain_template_stack(1, 1),
                        plain_template_stack(2, 1),
                    ],
                    Some(3),
                ),
                true,
            ),
            (
                container_patch(
                    vec![plain_template_stack(0, 1), plain_template_stack(2, 1)],
                    Some(2),
                ),
                false,
            ),
        ],
    );
}

#[test]
fn native_item_runtime_resolves_component_condition_exact_and_partial_predicates() {
    let h = ComponentConditionHarness::new();
    let healing_potion_id = 24;
    let star_trail = patch_with(&[68], |p| {
        p.firework_explosion = Some(bbb_protocol::packets::FireworkExplosionSummary {
            shape: FireworkExplosionShapeSummary::Star,
            has_trail: true,
            has_twinkle: false,
            ..Default::default()
        });
    });
    let fireworks_explosion =
        |shape: FireworkExplosionShapeSummary, has_trail: bool, has_twinkle: bool| {
            FireworkExplosionSummary {
                shape,
                colors: Vec::new(),
                fade_colors: Vec::new(),
                has_trail,
                has_twinkle,
            }
        };
    h.assert_cases(
        10,
        "component_condition_firework_explosion_star",
        [
            (star_trail.clone(), true),
            (
                DataComponentPatchSummary {
                    firework_explosion: Some(bbb_protocol::packets::FireworkExplosionSummary {
                        shape: FireworkExplosionShapeSummary::Burst,
                        ..star_trail.firework_explosion.clone().unwrap()
                    }),
                    ..star_trail.clone()
                },
                false,
            ),
            (
                DataComponentPatchSummary {
                    firework_explosion: Some(bbb_protocol::packets::FireworkExplosionSummary {
                        has_twinkle: true,
                        ..star_trail.firework_explosion.clone().unwrap()
                    }),
                    ..star_trail.clone()
                },
                false,
            ),
            (patch_with(&[68], |_| {}), false),
        ],
    );

    h.assert_cases(
        11,
        "component_condition_fireworks_flight",
        [
            (
                patch_with(&[69], |p| p.fireworks_flight_duration = Some(2)),
                true,
            ),
            (
                patch_with(&[69], |p| p.fireworks_flight_duration = Some(4)),
                false,
            ),
            (patch_with(&[69], |_| {}), false),
            (
                patch_with(&[69], |p| {
                    p.removed_type_ids = vec![69];
                    p.fireworks_flight_duration = Some(2);
                }),
                false,
            ),
        ],
    );
    h.assert_cases(
        12,
        "component_condition_fireworks_explosions",
        [
            (
                patch_with(&[69], |p| {
                    p.fireworks_flight_duration = Some(2);
                    p.fireworks_explosions = vec![Default::default(); 1];
                }),
                true,
            ),
            (
                patch_with(&[69], |p| {
                    p.fireworks_flight_duration = Some(2);
                    p.fireworks_explosions = vec![Default::default(); 2];
                }),
                false,
            ),
            (
                patch_with(&[69], |p| p.fireworks_flight_duration = Some(2)),
                false,
            ),
            (
                patch_with(&[69], |p| {
                    p.removed_type_ids = vec![69];
                    p.fireworks_flight_duration = Some(2);
                    p.fireworks_explosions = vec![Default::default(); 1];
                }),
                false,
            ),
        ],
    );
    h.assert_cases(
        19,
        "component_condition_fireworks_contains",
        [
            (
                patch_with(&[69], |p| {
                    p.fireworks_flight_duration = Some(1);
                    p.fireworks_explosions = vec![
                        fireworks_explosion(FireworkExplosionShapeSummary::SmallBall, false, false),
                        fireworks_explosion(FireworkExplosionShapeSummary::Star, true, false),
                    ];
                }),
                true,
            ),
            (
                patch_with(&[69], |p| {
                    p.fireworks_flight_duration = Some(1);
                    p.fireworks_explosions = vec![fireworks_explosion(
                        FireworkExplosionShapeSummary::Star,
                        false,
                        false,
                    )];
                }),
                false,
            ),
            (
                patch_with(&[69], |p| {
                    p.removed_type_ids = vec![69];
                    p.fireworks_flight_duration = Some(1);
                    p.fireworks_explosions = vec![fireworks_explosion(
                        FireworkExplosionShapeSummary::Star,
                        true,
                        false,
                    )];
                }),
                false,
            ),
        ],
    );
    h.assert_cases(
        20,
        "component_condition_fireworks_count",
        [
            (
                patch_with(&[69], |p| {
                    p.fireworks_flight_duration = Some(1);
                    p.fireworks_explosions = vec![
                        fireworks_explosion(FireworkExplosionShapeSummary::Star, true, true),
                        fireworks_explosion(FireworkExplosionShapeSummary::Burst, false, true),
                        fireworks_explosion(FireworkExplosionShapeSummary::SmallBall, false, false),
                    ];
                }),
                true,
            ),
            (
                patch_with(&[69], |p| {
                    p.fireworks_flight_duration = Some(1);
                    p.fireworks_explosions = vec![
                        fireworks_explosion(FireworkExplosionShapeSummary::Star, true, true),
                        fireworks_explosion(FireworkExplosionShapeSummary::SmallBall, false, false),
                    ];
                }),
                false,
            ),
        ],
    );

    h.assert_cases(
        13,
        "component_condition_trim_material",
        [(DataComponentPatchSummary::default(), false)],
    );
    h.assert_trim_cases(
        13,
        "component_condition_trim_material",
        [
            (
                patch_with(&[56], |p| p.armor_trim_material_id = Some(1)),
                true,
            ),
            (
                patch_with(&[56], |p| p.armor_trim_material_id = Some(0)),
                false,
            ),
        ],
    );
    h.assert_cases(
        13,
        "component_condition_trim_material",
        [(
            patch_with(&[56], |p| p.armor_trim_material_id = Some(1)),
            false,
        )],
    );
    h.assert_trim_cases(
        13,
        "component_condition_trim_material",
        [(
            patch_with(&[56], |p| {
                p.removed_type_ids = vec![56];
                p.armor_trim_material_id = Some(1);
            }),
            false,
        )],
    );
    h.assert_trim_cases(
        14,
        "component_condition_trim_pattern",
        [
            (
                patch_with(&[56], |p| {
                    p.armor_trim_material_id = Some(1);
                    p.armor_trim_pattern_id = Some(0);
                }),
                true,
            ),
            (
                patch_with(&[56], |p| p.armor_trim_material_id = Some(1)),
                false,
            ),
        ],
    );
    let reordered_trim_pattern_keys = [
        "minecraft:spire".to_string(),
        "minecraft:sentry".to_string(),
    ];
    assert_eq!(
        h.selected_with_registry_trim_keys(
            14,
            patch_with(&[56], |p| {
                p.armor_trim_material_id = Some(1);
                p.armor_trim_pattern_id = Some(0);
            }),
            &h.trim_keys,
            &reordered_trim_pattern_keys,
        ),
        h.uv("component_condition_trim_pattern_absent")
    );
    assert_eq!(
        h.selected_with_registry_trim_keys(
            14,
            patch_with(&[56], |p| {
                p.armor_trim_material_id = Some(1);
                p.armor_trim_pattern_id = Some(1);
            }),
            &h.trim_keys,
            &reordered_trim_pattern_keys,
        ),
        h.uv("component_condition_trim_pattern_present")
    );
    h.assert_trim_cases(
        14,
        "component_condition_trim_pattern",
        [
            (
                patch_with(&[56], |p| {
                    p.armor_trim_material_id = Some(1);
                    p.armor_trim_pattern_id = Some(1);
                }),
                false,
            ),
            (
                patch_with(&[56], |p| {
                    p.removed_type_ids = vec![56];
                    p.armor_trim_material_id = Some(1);
                    p.armor_trim_pattern_id = Some(0);
                }),
                false,
            ),
        ],
    );
    h.assert_trim_cases(
        34,
        "component_condition_trim_material_tag",
        [
            (
                patch_with(&[56], |p| p.armor_trim_material_id = Some(1)),
                true,
            ),
            (
                patch_with(&[56], |p| p.armor_trim_material_id = Some(0)),
                false,
            ),
            (
                patch_with(&[56], |p| {
                    p.removed_type_ids = vec![56];
                    p.armor_trim_material_id = Some(1);
                }),
                false,
            ),
        ],
    );
    h.assert_trim_cases(
        35,
        "component_condition_trim_pattern_tag",
        [
            (
                patch_with(&[56], |p| {
                    p.armor_trim_material_id = Some(1);
                    p.armor_trim_pattern_id = Some(0);
                }),
                true,
            ),
            (
                patch_with(&[56], |p| {
                    p.armor_trim_material_id = Some(1);
                    p.armor_trim_pattern_id = Some(1);
                }),
                false,
            ),
            (
                patch_with(&[56], |p| {
                    p.removed_type_ids = vec![56];
                    p.armor_trim_material_id = Some(1);
                    p.armor_trim_pattern_id = Some(0);
                }),
                false,
            ),
        ],
    );

    h.assert_cases(
        15,
        "component_condition_enchantments_level",
        [
            (DataComponentPatchSummary::default(), false),
            (
                patch_with(&[13], |p| {
                    p.enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
                        holder_id: 7,
                        level: 3,
                    }];
                }),
                true,
            ),
            (
                patch_with(&[13], |p| {
                    p.enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
                        holder_id: 7,
                        level: 1,
                    }];
                }),
                false,
            ),
            (patch_with(&[13], |_| {}), false),
            (
                patch_with(&[13], |p| {
                    p.removed_type_ids = vec![13];
                    p.enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
                        holder_id: 7,
                        level: 3,
                    }];
                }),
                false,
            ),
        ],
    );
    h.assert_cases(
        16,
        "component_condition_enchantments_empty",
        [
            (DataComponentPatchSummary::default(), true),
            (patch_removed(&[13]), false),
        ],
    );

    h.assert_cases(
        17,
        "component_condition_stored_enchantments_level",
        [
            (DataComponentPatchSummary::default(), false),
            (
                patch_with(&[42], |p| {
                    p.stored_enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
                        holder_id: 11,
                        level: 3,
                    }];
                }),
                true,
            ),
            (
                patch_with(&[42], |p| {
                    p.stored_enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
                        holder_id: 11,
                        level: 1,
                    }];
                }),
                false,
            ),
            (
                patch_with(&[42], |p| {
                    p.removed_type_ids = vec![42];
                    p.stored_enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
                        holder_id: 11,
                        level: 3,
                    }];
                }),
                false,
            ),
        ],
    );
    h.assert_cases(
        18,
        "component_condition_stored_enchantments_empty",
        [
            (DataComponentPatchSummary::default(), false),
            (patch_with(&[42], |_| {}), true),
        ],
    );
    h.assert_cases(
        27,
        "component_condition_stored_enchantments_default",
        [
            (DataComponentPatchSummary::default(), true),
            (patch_removed(&[42]), false),
        ],
    );
    h.assert_cases(
        25,
        "component_condition_enchantments_holder",
        [(
            patch_with(&[13], |p| {
                p.enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
                    holder_id: 0,
                    level: 3,
                }];
            }),
            false,
        )],
    );
    h.assert_enchant_cases(
        25,
        "component_condition_enchantments_holder",
        [
            (
                patch_with(&[13], |p| {
                    p.enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
                        holder_id: 0,
                        level: 3,
                    }];
                }),
                true,
            ),
            (
                patch_with(&[13], |p| {
                    p.enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
                        holder_id: 1,
                        level: 3,
                    }];
                }),
                false,
            ),
            (
                patch_with(&[13], |p| {
                    p.enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
                        holder_id: 0,
                        level: 1,
                    }];
                }),
                false,
            ),
            (
                patch_with(&[13], |p| {
                    p.removed_type_ids = vec![13];
                    p.enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
                        holder_id: 0,
                        level: 3,
                    }];
                }),
                false,
            ),
        ],
    );
    h.assert_enchant_cases(
        26,
        "component_condition_stored_enchantments_holder",
        [
            (
                patch_with(&[42], |p| {
                    p.stored_enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
                        holder_id: 1,
                        level: 1,
                    }];
                }),
                true,
            ),
            (
                patch_with(&[42], |p| {
                    p.stored_enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
                        holder_id: 0,
                        level: 1,
                    }];
                }),
                false,
            ),
            (
                patch_with(&[42], |p| {
                    p.stored_enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
                        holder_id: 1,
                        level: 2,
                    }];
                }),
                false,
            ),
            (
                patch_with(&[42], |p| {
                    p.removed_type_ids = vec![42];
                    p.stored_enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
                        holder_id: 1,
                        level: 1,
                    }];
                }),
                false,
            ),
        ],
    );
    h.assert_cases(
        30,
        "component_condition_enchantments_tag",
        [(
            patch_with(&[13], |p| {
                p.enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
                    holder_id: 0,
                    level: 3,
                }];
            }),
            false,
        )],
    );
    h.assert_enchant_cases(
        30,
        "component_condition_enchantments_tag",
        [
            (
                patch_with(&[13], |p| {
                    p.enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
                        holder_id: 0,
                        level: 3,
                    }];
                }),
                true,
            ),
            (
                patch_with(&[13], |p| {
                    p.enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
                        holder_id: 1,
                        level: 3,
                    }];
                }),
                false,
            ),
            (
                patch_with(&[13], |p| {
                    p.enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
                        holder_id: 0,
                        level: 1,
                    }];
                }),
                false,
            ),
        ],
    );
    h.assert_enchant_cases(
        31,
        "component_condition_stored_enchantments_tag",
        [
            (
                patch_with(&[42], |p| {
                    p.stored_enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
                        holder_id: 0,
                        level: 1,
                    }];
                }),
                true,
            ),
            (
                patch_with(&[42], |p| {
                    p.stored_enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
                        holder_id: 1,
                        level: 1,
                    }];
                }),
                false,
            ),
        ],
    );
    h.assert_cases(
        32,
        "component_condition_bundle_components",
        [
            (
                bundle_patch(vec![template_stack(
                    0,
                    1,
                    patch_with(
                        &[component_id::VANILLA_DATA_COMPONENT_RARITY_TYPE_ID],
                        |p| p.rarity = Some(ItemRaritySummary::Rare),
                    ),
                )]),
                true,
            ),
            (bundle_patch(vec![plain_template_stack(0, 1)]), false),
            (
                bundle_patch(vec![template_stack(
                    0,
                    1,
                    DataComponentPatchSummary {
                        rarity: Some(ItemRaritySummary::Rare),
                        removed_type_ids: vec![12],
                        ..DataComponentPatchSummary::default()
                    },
                )]),
                false,
            ),
        ],
    );
    let named_bundle_entry = |component_patch: DataComponentPatchSummary| {
        bundle_patch(vec![template_stack(0, 1, component_patch)])
    };
    h.assert_cases(
        81,
        "component_condition_bundle_exact_component_text",
        [
            (
                named_bundle_entry(patch_with(&[6, 9], |p| {
                    p.custom_name = Some("Custom Name".to_string());
                    p.item_name = Some("Item Name".to_string());
                })),
                true,
            ),
            (
                named_bundle_entry(patch_with(&[6], |p| {
                    p.custom_name = Some("Custom Name".to_string())
                })),
                false,
            ),
            (
                named_bundle_entry(patch_with(&[6, 9], |p| {
                    p.removed_type_ids = vec![6];
                    p.custom_name = Some("Custom Name".to_string());
                    p.item_name = Some("Item Name".to_string());
                })),
                false,
            ),
        ],
    );
    h.assert_cases(
        82,
        "component_condition_bundle_exact_lore",
        [
            (
                named_bundle_entry(patch_with(&[11], |p| {
                    p.lore = vec!["Lore one".to_string(), "Lore two".to_string()]
                })),
                true,
            ),
            (
                named_bundle_entry(patch_with(&[11], |p| {
                    p.lore = vec!["Lore two".to_string(), "Lore one".to_string()]
                })),
                false,
            ),
            (
                named_bundle_entry(patch_with(&[11], |p| {
                    p.removed_type_ids = vec![11];
                    p.lore = vec!["Lore one".to_string(), "Lore two".to_string()];
                })),
                false,
            ),
        ],
    );
    h.assert_cases(
        83,
        "component_condition_bundle_exact_unbreakable",
        [
            (
                named_bundle_entry(patch_with(&[4], |p| p.unbreakable = true)),
                true,
            ),
            (
                named_bundle_entry(DataComponentPatchSummary::default()),
                false,
            ),
            (
                named_bundle_entry(patch_with(&[4], |p| {
                    p.removed_type_ids = vec![4];
                    p.unbreakable = true;
                })),
                false,
            ),
        ],
    );
    let exact_custom_data = NbtSummaryValue::Compound(vec![
        NbtSummaryEntry {
            name: "owner".into(),
            value: NbtSummaryValue::string("Alex"),
        },
        NbtSummaryEntry {
            name: "level".into(),
            value: NbtSummaryValue::Int(7),
        },
    ]);
    h.assert_cases(
        84,
        "component_condition_bundle_exact_custom_data",
        [
            (
                named_bundle_entry(patch_with(&[0], |p| {
                    p.custom_data = Some(exact_custom_data.clone())
                })),
                true,
            ),
            (
                named_bundle_entry(patch_with(&[0], |p| {
                    p.custom_data = Some(NbtSummaryValue::Compound(vec![
                        NbtSummaryEntry {
                            name: "owner".into(),
                            value: NbtSummaryValue::string("Alex"),
                        },
                        NbtSummaryEntry {
                            name: "level".into(),
                            value: NbtSummaryValue::Int(7),
                        },
                        NbtSummaryEntry {
                            name: "extra".into(),
                            value: NbtSummaryValue::Byte(1),
                        },
                    ]));
                })),
                false,
            ),
            (
                named_bundle_entry(patch_with(&[0], |p| {
                    p.removed_type_ids = vec![0];
                    p.custom_data = Some(exact_custom_data);
                })),
                false,
            ),
        ],
    );
    let exact_potion_effect = MobEffectInstanceSummary {
        effect_id: VANILLA_MOB_EFFECT_HASTE_ID,
        amplifier: 1,
        duration: 200,
        ambient: false,
        show_particles: true,
        show_icon: true,
        hidden_effect: Some(Box::new(MobEffectDetailsSummary {
            amplifier: 0,
            duration: 40,
            ambient: false,
            show_particles: true,
            show_icon: false,
            hidden_effect: None,
        })),
    };
    let exact_potion_contents = patch_with(&[51], |p| {
        p.potion_contents = bbb_protocol::packets::PotionContentsSummary {
            potion_id: Some(healing_potion_id),
            custom_color: Some(1),
            custom_effects: vec![exact_potion_effect.clone()],
            custom_name: Some("healing".into()),
        };
    });
    h.assert_cases(
        85,
        "component_condition_bundle_exact_potion_contents",
        [(named_bundle_entry(exact_potion_contents.clone()), true)],
    );
    for hidden_effect in [
        None,
        Some(Box::new(MobEffectDetailsSummary {
            duration: 999,
            ..exact_potion_effect
                .hidden_effect
                .as_deref()
                .unwrap()
                .clone()
        })),
    ] {
        assert_eq!(
            h.selected(
                85,
                named_bundle_entry(DataComponentPatchSummary {
                    potion_contents: bbb_protocol::packets::PotionContentsSummary {
                        custom_effects: vec![MobEffectInstanceSummary {
                            hidden_effect,
                            ..exact_potion_effect.clone()
                        }],
                        ..exact_potion_contents.potion_contents.clone()
                    },
                    ..exact_potion_contents.clone()
                })
            ),
            h.uv("component_condition_bundle_exact_potion_contents_present")
        );
    }
    h.assert_cases(
        85,
        "component_condition_bundle_exact_potion_contents",
        [
            (
                named_bundle_entry(DataComponentPatchSummary {
                    potion_contents: bbb_protocol::packets::PotionContentsSummary {
                        custom_effects: vec![MobEffectInstanceSummary {
                            duration: 201,
                            ..exact_potion_effect.clone()
                        }],
                        ..exact_potion_contents.potion_contents.clone()
                    },
                    ..exact_potion_contents.clone()
                }),
                false,
            ),
            (
                named_bundle_entry(DataComponentPatchSummary {
                    potion_contents: bbb_protocol::packets::PotionContentsSummary {
                        custom_effects: vec![MobEffectInstanceSummary {
                            show_particles: false,
                            ..exact_potion_effect
                        }],
                        ..exact_potion_contents.potion_contents.clone()
                    },
                    ..exact_potion_contents.clone()
                }),
                false,
            ),
            (
                named_bundle_entry(DataComponentPatchSummary {
                    removed_type_ids: vec![51],
                    ..exact_potion_contents
                }),
                false,
            ),
        ],
    );
    let exact_writable_book = patch_with(&[54], |p| {
        p.writable_book_pages = vec!["alpha".into(), "beta".into()];
        p.writable_book_page_filters = vec![None, Some("filtered beta".into())];
    });
    h.assert_cases(
        86,
        "component_condition_bundle_exact_writable_book",
        [
            (named_bundle_entry(exact_writable_book.clone()), true),
            (
                named_bundle_entry(DataComponentPatchSummary {
                    writable_book_page_filters: vec![None, Some("other filtered".into())],
                    ..exact_writable_book.clone()
                }),
                false,
            ),
            (
                named_bundle_entry(DataComponentPatchSummary {
                    removed_type_ids: vec![54],
                    ..exact_writable_book
                }),
                false,
            ),
        ],
    );
    let exact_firework_explosion = patch_with(&[68], |p| {
        p.firework_explosion = Some(bbb_protocol::packets::FireworkExplosionSummary {
            shape: FireworkExplosionShapeSummary::Star,
            colors: vec![0x11_22_33],
            fade_colors: vec![0x44_55_66],
            has_trail: true,
            has_twinkle: false,
        });
    });
    h.assert_cases(
        87,
        "component_condition_bundle_exact_firework_explosion",
        [
            (named_bundle_entry(exact_firework_explosion.clone()), true),
            (
                named_bundle_entry(DataComponentPatchSummary {
                    firework_explosion: Some(bbb_protocol::packets::FireworkExplosionSummary {
                        fade_colors: vec![0x01_02_03],
                        ..exact_firework_explosion.firework_explosion.clone().unwrap()
                    }),
                    ..exact_firework_explosion.clone()
                }),
                false,
            ),
            (
                named_bundle_entry(DataComponentPatchSummary {
                    removed_type_ids: vec![68],
                    ..exact_firework_explosion
                }),
                false,
            ),
        ],
    );
    let exact_fireworks = patch_with(&[69], |p| {
        p.fireworks_flight_duration = Some(2);
        p.fireworks_explosions = vec![FireworkExplosionSummary {
            shape: FireworkExplosionShapeSummary::Star,
            colors: vec![0x11_22_33],
            fade_colors: vec![0x44_55_66],
            has_trail: true,
            has_twinkle: false,
        }];
    });
    h.assert_cases(
        88,
        "component_condition_bundle_exact_fireworks",
        [
            (named_bundle_entry(exact_fireworks.clone()), true),
            (
                named_bundle_entry(DataComponentPatchSummary {
                    fireworks_flight_duration: Some(1),
                    ..exact_fireworks.clone()
                }),
                false,
            ),
            (
                named_bundle_entry(DataComponentPatchSummary {
                    fireworks_explosions: vec![FireworkExplosionSummary {
                        shape: FireworkExplosionShapeSummary::Star,
                        colors: vec![0x11_22_33],
                        fade_colors: vec![0x01_02_03],
                        has_trail: true,
                        has_twinkle: false,
                    }],
                    ..exact_fireworks
                }),
                false,
            ),
        ],
    );
    let exact_jukebox_playable = patch_with(&[64], |p| {
        p.jukebox_direct_song = Some(Box::new(JukeboxSongSummary {
            sound_event: SoundEventSummary {
                registry_id: Some(286),
                sound_id: None,
                fixed_range_bits: None,
            },
            description: "Test song".to_string(),
            description_styled: None,
            description_nbt: None,
            length_in_seconds_bits: 3.5f32.to_bits(),
            comparator_output: 7,
        }));
    });
    h.assert_cases(
        89,
        "component_condition_bundle_exact_jukebox_playable",
        [
            (named_bundle_entry(exact_jukebox_playable.clone()), true),
            (
                named_bundle_entry(DataComponentPatchSummary {
                    jukebox_direct_song: Some(Box::new(JukeboxSongSummary {
                        comparator_output: 8,
                        ..exact_jukebox_playable
                            .jukebox_direct_song
                            .clone()
                            .unwrap()
                            .as_ref()
                            .clone()
                    })),
                    ..exact_jukebox_playable.clone()
                }),
                false,
            ),
            (
                named_bundle_entry(DataComponentPatchSummary {
                    removed_type_ids: vec![64],
                    ..exact_jukebox_playable
                }),
                false,
            ),
        ],
    );
    let exact_trim = patch_with(&[56], |p| {
        p.armor_trim_material_direct = Some(Box::new(TrimMaterialSummary {
            asset_name: "test_material".to_string(),
            override_armor_assets: BTreeMap::from([(
                "minecraft:iron".to_string(),
                "test_material_iron".to_string(),
            )]),
            description: "Test material".to_string(),
            description_styled: None,
            description_nbt: None,
        }));
        p.armor_trim_pattern_direct = Some(Box::new(TrimPatternSummary {
            asset_id: "minecraft:test_pattern".to_string(),
            description: "Test pattern".to_string(),
            description_styled: None,
            description_nbt: None,
            decal: true,
        }));
    });
    h.assert_trim_cases(
        90,
        "component_condition_bundle_exact_trim",
        [
            (named_bundle_entry(exact_trim.clone()), true),
            (
                named_bundle_entry(DataComponentPatchSummary {
                    armor_trim_pattern_direct: Some(Box::new(TrimPatternSummary {
                        decal: false,
                        ..exact_trim
                            .armor_trim_pattern_direct
                            .clone()
                            .unwrap()
                            .as_ref()
                            .clone()
                    })),
                    ..exact_trim.clone()
                }),
                false,
            ),
            (
                named_bundle_entry(DataComponentPatchSummary {
                    armor_trim_material_direct: Some(Box::new(TrimMaterialSummary {
                        description: "Other material".to_string(),
                        ..exact_trim
                            .armor_trim_material_direct
                            .clone()
                            .unwrap()
                            .as_ref()
                            .clone()
                    })),
                    ..exact_trim.clone()
                }),
                false,
            ),
            (
                named_bundle_entry(DataComponentPatchSummary {
                    removed_type_ids: vec![56],
                    ..exact_trim
                }),
                false,
            ),
        ],
    );
    let exact_enchantments = patch_with(&[13], |p| {
        p.enchantments = vec![
            bbb_protocol::packets::ItemEnchantmentSummary {
                holder_id: 0,
                level: 3,
            },
            bbb_protocol::packets::ItemEnchantmentSummary {
                holder_id: 1,
                level: 1,
            },
        ];
    });
    h.assert_enchant_cases(
        91,
        "component_condition_bundle_exact_enchantments",
        [(named_bundle_entry(exact_enchantments.clone()), true)],
    );
    h.assert_cases(
        91,
        "component_condition_bundle_exact_enchantments",
        [(named_bundle_entry(exact_enchantments.clone()), false)],
    );
    h.assert_enchant_cases(
        91,
        "component_condition_bundle_exact_enchantments",
        [
            (
                named_bundle_entry(DataComponentPatchSummary {
                    enchantments: vec![
                        bbb_protocol::packets::ItemEnchantmentSummary {
                            holder_id: 0,
                            level: 3,
                        },
                        bbb_protocol::packets::ItemEnchantmentSummary {
                            holder_id: 1,
                            level: 2,
                        },
                    ],
                    ..exact_enchantments.clone()
                }),
                false,
            ),
            (
                named_bundle_entry(DataComponentPatchSummary {
                    removed_type_ids: vec![13],
                    ..exact_enchantments
                }),
                false,
            ),
        ],
    );
    let exact_stored_enchantments = patch_with(&[42], |p| {
        p.stored_enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
            holder_id: 1,
            level: 1,
        }];
    });
    h.assert_enchant_cases(
        92,
        "component_condition_bundle_exact_stored_enchantments",
        [
            (named_bundle_entry(exact_stored_enchantments.clone()), true),
            (
                named_bundle_entry(DataComponentPatchSummary {
                    stored_enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                        holder_id: 0,
                        level: 1,
                    }],
                    ..exact_stored_enchantments.clone()
                }),
                false,
            ),
            (
                named_bundle_entry(DataComponentPatchSummary {
                    removed_type_ids: vec![42],
                    ..exact_stored_enchantments
                }),
                false,
            ),
        ],
    );
    let exact_villager_variant = patch_with(&[81], |p| p.villager_variant_id = Some(2));
    h.assert_cases(
        93,
        "component_condition_bundle_exact_villager_variant",
        [
            (named_bundle_entry(exact_villager_variant.clone()), true),
            (
                named_bundle_entry(DataComponentPatchSummary {
                    villager_variant_id: Some(0),
                    ..exact_villager_variant.clone()
                }),
                false,
            ),
            (
                named_bundle_entry(DataComponentPatchSummary {
                    removed_type_ids: vec![81],
                    ..exact_villager_variant
                }),
                false,
            ),
        ],
    );
    let exact_lodestone_tracker = patch_with(&[67], |p| {
        p.lodestone_target = Some(LodestoneTargetSummary {
            dimension: "minecraft:overworld".to_string(),
            pos: bbb_protocol::packets::chunks::BlockPos {
                x: 10,
                y: 64,
                z: -4,
            },
        });
        p.lodestone_tracked = Some(false);
    });
    h.assert_cases(
        94,
        "component_condition_bundle_exact_lodestone_tracker",
        [
            (named_bundle_entry(exact_lodestone_tracker.clone()), true),
            (
                named_bundle_entry(DataComponentPatchSummary {
                    lodestone_tracked: Some(true),
                    ..exact_lodestone_tracker.clone()
                }),
                false,
            ),
            (
                named_bundle_entry(DataComponentPatchSummary {
                    lodestone_target: Some(LodestoneTargetSummary {
                        dimension: "minecraft:the_nether".to_string(),
                        ..exact_lodestone_tracker.lodestone_target.clone().unwrap()
                    }),
                    ..exact_lodestone_tracker.clone()
                }),
                false,
            ),
            (
                named_bundle_entry(DataComponentPatchSummary {
                    removed_type_ids: vec![67],
                    ..exact_lodestone_tracker
                }),
                false,
            ),
        ],
    );
    let exact_attribute_modifiers = patch_with(&[16], |p| {
        p.attribute_modifiers = vec![
            AttributeModifierSummary {
                attribute_id: i32::try_from(h.attribute_keys.iter().position(|key| key == "minecraft:attack_damage").unwrap()).unwrap(),
                modifier_id: "minecraft:test/attack".to_string(),
                amount_bits: 2.5f64.to_bits(),
                operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
                slot_id: 1,
                display_id: 1,
                display_text: None,
                display_text_styled: None,
                display_text_nbt: None,
            },
            AttributeModifierSummary {
                attribute_id: i32::try_from(h.attribute_keys.iter().position(|key| key == "minecraft:scale").unwrap()).unwrap(),
                modifier_id: "minecraft:test/scale".to_string(),
                amount_bits: 1.25f64.to_bits(),
                operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
                slot_id: 3,
                display_id: 2,
                display_text: Some("Heavy hit".to_string()),
                display_text_styled: None,
                display_text_nbt: None,
            },
        ];
    });
    let attribute_defaults = build_component_defaults(BTreeMap::from([(
        0,
        BTreeMap::from([(
            component_id::VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID,
            serde_json::json!([
                {
                    "type": "minecraft:attack_damage", "id": "minecraft:test/attack",
                    "amount": 2.5, "operation": "add_value", "slot": "mainhand",
                    "display": {"type": "hidden"}
                },
                {
                    "type": "minecraft:scale", "id": "minecraft:test/scale",
                    "amount": 1.25, "operation": "add_multiplied_base", "slot": "hand",
                    "display": {"type": "override", "value": {"text": "Heavy hit"}}
                }
            ]),
        )]),
    )]));
    for (component_patch, expected_model) in [
        (DataComponentPatchSummary::default(), "present"),
        (patch_with(&[16], |_| {}), "absent"),
        (
            DataComponentPatchSummary {
                removed_type_ids: vec![16],
                ..exact_attribute_modifiers.clone()
            },
            "absent",
        ),
    ] {
        let model = h
            .runtime
            .resolve_item_model(
                &ItemStackSummary {
                    item_id: Some(95),
                    count: 1,
                    component_patch: named_bundle_entry(component_patch).into(),
                },
                &ItemModelContext {
                    item_profiles: Some(attribute_defaults.as_view()),
                    registry_keys: ItemModelRegistryKeys {
                        attributes: Some(&h.attribute_keys),
                        ..Default::default()
                    },
                    ..h.base_context()
                },
            )
            .unwrap();
        assert_eq!(
            model.flat_icon().unwrap().layers[0].uv,
            h.uv(&format!(
                "component_condition_bundle_exact_attribute_modifiers_{expected_model}"
            ))
        );
    }
    h.assert_attr_cases(
        95,
        "component_condition_bundle_exact_attribute_modifiers",
        [
            (named_bundle_entry(exact_attribute_modifiers.clone()), true),
            (
                named_bundle_entry(DataComponentPatchSummary {
                    attribute_modifiers: vec![
                        AttributeModifierSummary {
                            display_id: 0,
                            display_text: None,
                            ..exact_attribute_modifiers.attribute_modifiers[0].clone()
                        },
                        exact_attribute_modifiers.attribute_modifiers[1].clone(),
                    ],
                    ..exact_attribute_modifiers.clone()
                }),
                false,
            ),
            (
                named_bundle_entry(DataComponentPatchSummary {
                    attribute_modifiers: vec![
                        AttributeModifierSummary {
                            amount_bits: 3.0f64.to_bits(),
                            display_text: None,
                            ..exact_attribute_modifiers.attribute_modifiers[0].clone()
                        },
                        exact_attribute_modifiers.attribute_modifiers[1].clone(),
                    ],
                    ..exact_attribute_modifiers.clone()
                }),
                false,
            ),
            (
                named_bundle_entry(DataComponentPatchSummary {
                    attribute_modifiers: vec![
                        exact_attribute_modifiers.attribute_modifiers[0].clone(),
                        AttributeModifierSummary {
                            display_text: Some("Light hit".to_string()),
                            ..exact_attribute_modifiers.attribute_modifiers[1].clone()
                        },
                    ],
                    ..exact_attribute_modifiers.clone()
                }),
                false,
            ),
            (
                named_bundle_entry(DataComponentPatchSummary {
                    removed_type_ids: vec![16],
                    ..exact_attribute_modifiers
                }),
                false,
            ),
        ],
    );
    let exact_written_book = patch_with(&[55], |p| {
        p.written_book = Some(WrittenBookContentSummary {
            title: "Quest".into(),
            title_filter: Some("Filtered Quest".into()),
            author: "Alex".into(),
            generation: 1,
            pages: vec!["First page".to_string(), "Second page".to_string()],
            page_filters: vec![None, Some("Filtered second page".to_string())],
            resolved: true,
        });
    });
    h.assert_cases(
        96,
        "component_condition_bundle_exact_written_book",
        [
            (named_bundle_entry(exact_written_book.clone()), true),
            (
                named_bundle_entry(DataComponentPatchSummary {
                    written_book: Some(WrittenBookContentSummary {
                        title_filter: None,
                        ..exact_written_book.written_book.clone().unwrap()
                    }),
                    ..exact_written_book.clone()
                }),
                false,
            ),
            (
                named_bundle_entry(DataComponentPatchSummary {
                    written_book: Some(WrittenBookContentSummary {
                        page_filters: vec![None, None],
                        ..exact_written_book.written_book.clone().unwrap()
                    }),
                    ..exact_written_book.clone()
                }),
                false,
            ),
            (
                named_bundle_entry(DataComponentPatchSummary {
                    removed_type_ids: vec![55],
                    ..exact_written_book
                }),
                false,
            ),
        ],
    );
    h.assert_cases(
        33,
        "component_condition_container_components",
        [
            (
                container_patch(
                    vec![plain_template_stack(0, 1), plain_template_stack(1, 1)],
                    Some(2),
                ),
                true,
            ),
            (
                container_patch(
                    vec![
                        template_stack(
                            0,
                            1,
                            patch_with(
                                &[component_id::VANILLA_DATA_COMPONENT_RARITY_TYPE_ID],
                                |p| p.rarity = Some(ItemRaritySummary::Rare),
                            ),
                        ),
                        plain_template_stack(1, 1),
                    ],
                    Some(2),
                ),
                false,
            ),
        ],
    );
    h.assert_cases(
        36,
        "component_condition_bundle_partial_damage",
        [
            (
                bundle_patch(vec![template_stack(
                    2,
                    1,
                    patch_with(&[3], |p| p.damage = Some(3)),
                )]),
                true,
            ),
            (
                bundle_patch(vec![template_stack(
                    2,
                    1,
                    patch_with(&[3], |p| p.damage = Some(4)),
                )]),
                false,
            ),
            (
                patch_with(&[50], |p| {
                    p.removed_type_ids = vec![50];
                    p.bundle_contents_items = vec![template_stack(
                        2,
                        1,
                        patch_with(&[3], |p| p.damage = Some(3)),
                    )];
                }),
                false,
            ),
        ],
    );
    h.assert_cases(
        37,
        "component_condition_container_partial_damage",
        [
            (
                container_patch(
                    vec![
                        template_stack(2, 1, patch_with(&[3], |p| p.damage = Some(3))),
                        template_stack(2, 1, patch_with(&[3], |p| p.damage = Some(3))),
                    ],
                    Some(2),
                ),
                true,
            ),
            (
                container_patch(
                    vec![
                        template_stack(2, 1, patch_with(&[3], |p| p.damage = Some(3))),
                        template_stack(2, 1, patch_with(&[3], |p| p.damage = Some(4))),
                    ],
                    Some(2),
                ),
                false,
            ),
        ],
    );
    h.assert_cases(
        38,
        "component_condition_bundle_partial_any_value",
        [
            (bundle_patch(vec![plain_template_stack(0, 1)]), true),
            (
                bundle_patch(vec![template_stack(0, 1, patch_removed(&[12]))]),
                false,
            ),
        ],
    );
    h.assert_cases(
        39,
        "component_condition_container_partial_any_value",
        [
            (
                container_patch(
                    vec![
                        template_stack(
                            0,
                            1,
                            patch_with(&[21], |p| p.enchantment_glint_override = Some(false)),
                        ),
                        template_stack(
                            1,
                            1,
                            patch_with(&[21], |p| p.enchantment_glint_override = Some(true)),
                        ),
                    ],
                    Some(2),
                ),
                true,
            ),
            (
                container_patch(
                    vec![
                        template_stack(
                            0,
                            1,
                            patch_with(&[21], |p| p.enchantment_glint_override = Some(false)),
                        ),
                        plain_template_stack(1, 1),
                    ],
                    Some(2),
                ),
                false,
            ),
        ],
    );
    h.assert_cases(
        40,
        "component_condition_bundle_partial_enchantments",
        [(
            bundle_patch(vec![template_stack(
                0,
                1,
                patch_with(&[13], |p| {
                    p.enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
                        holder_id: 0,
                        level: 3,
                    }];
                }),
            )]),
            false,
        )],
    );
    h.assert_enchant_cases(
        40,
        "component_condition_bundle_partial_enchantments",
        [
            (
                bundle_patch(vec![template_stack(
                    0,
                    1,
                    patch_with(&[13], |p| {
                        p.enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
                            holder_id: 0,
                            level: 3,
                        }];
                    }),
                )]),
                true,
            ),
            (
                bundle_patch(vec![template_stack(
                    0,
                    1,
                    patch_with(&[13], |p| {
                        p.enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
                            holder_id: 1,
                            level: 3,
                        }];
                    }),
                )]),
                false,
            ),
        ],
    );
    h.assert_enchant_cases(
        41,
        "component_condition_container_partial_stored_enchantments",
        [
            (
                container_patch(
                    vec![
                        template_stack(
                            0,
                            1,
                            patch_with(&[42], |p| {
                                p.stored_enchantments =
                                    vec![bbb_protocol::packets::ItemEnchantmentSummary {
                                        holder_id: 1,
                                        level: 1,
                                    }];
                            }),
                        ),
                        template_stack(
                            1,
                            1,
                            patch_with(&[42], |p| {
                                p.stored_enchantments =
                                    vec![bbb_protocol::packets::ItemEnchantmentSummary {
                                        holder_id: 1,
                                        level: 1,
                                    }];
                            }),
                        ),
                    ],
                    Some(2),
                ),
                true,
            ),
            (
                container_patch(
                    vec![
                        template_stack(
                            0,
                            1,
                            patch_with(&[42], |p| {
                                p.stored_enchantments =
                                    vec![bbb_protocol::packets::ItemEnchantmentSummary {
                                        holder_id: 1,
                                        level: 1,
                                    }];
                            }),
                        ),
                        template_stack(
                            1,
                            1,
                            patch_with(&[42], |p| {
                                p.stored_enchantments =
                                    vec![bbb_protocol::packets::ItemEnchantmentSummary {
                                        holder_id: 0,
                                        level: 1,
                                    }];
                            }),
                        ),
                    ],
                    Some(2),
                ),
                false,
            ),
        ],
    );
    h.assert_cases(
        42,
        "component_condition_bundle_partial_firework_explosion",
        [
            (
                bundle_patch(vec![template_stack(0, 1, star_trail.clone())]),
                true,
            ),
            (
                bundle_patch(vec![template_stack(
                    0,
                    1,
                    DataComponentPatchSummary {
                        firework_explosion: Some(bbb_protocol::packets::FireworkExplosionSummary {
                            shape: FireworkExplosionShapeSummary::Burst,
                            ..star_trail.firework_explosion.clone().unwrap()
                        }),
                        ..star_trail.clone()
                    },
                )]),
                false,
            ),
        ],
    );
    h.assert_cases(
        43,
        "component_condition_container_partial_fireworks",
        [
            (
                container_patch(
                    vec![
                        template_stack(
                            0,
                            1,
                            patch_with(&[69], |p| {
                                p.fireworks_flight_duration = Some(3);
                                p.fireworks_explosions = vec![fireworks_explosion(
                                    FireworkExplosionShapeSummary::Burst,
                                    false,
                                    true,
                                )];
                            }),
                        ),
                        template_stack(
                            1,
                            1,
                            patch_with(&[69], |p| {
                                p.fireworks_flight_duration = Some(2);
                                p.fireworks_explosions = vec![fireworks_explosion(
                                    FireworkExplosionShapeSummary::Burst,
                                    true,
                                    true,
                                )];
                            }),
                        ),
                    ],
                    Some(2),
                ),
                true,
            ),
            (
                container_patch(
                    vec![
                        template_stack(
                            0,
                            1,
                            patch_with(&[69], |p| {
                                p.fireworks_flight_duration = Some(3);
                                p.fireworks_explosions = vec![fireworks_explosion(
                                    FireworkExplosionShapeSummary::Burst,
                                    false,
                                    true,
                                )];
                            }),
                        ),
                        template_stack(
                            1,
                            1,
                            patch_with(&[69], |p| {
                                p.fireworks_flight_duration = Some(1);
                                p.fireworks_explosions = vec![fireworks_explosion(
                                    FireworkExplosionShapeSummary::Burst,
                                    true,
                                    true,
                                )];
                            }),
                        ),
                    ],
                    Some(2),
                ),
                false,
            ),
        ],
    );
}
