use super::*;

#[test]
fn native_item_runtime_resolves_component_condition_attribute_and_custom_data_predicates() {
    let h = ComponentConditionHarness::new();
    let healing_potion_id = 24;

    h.assert_cases(
        44,
        "component_condition_bundle_partial_trim",
        [(
            bundle_patch(vec![template_stack(
                0,
                1,
                patch_with(&[56], |p| p.armor_trim_material_id = Some(1)),
            )]),
            false,
        )],
    );
    h.assert_trim_cases(
        44,
        "component_condition_bundle_partial_trim",
        [
            (
                bundle_patch(vec![template_stack(
                    0,
                    1,
                    patch_with(&[56], |p| p.armor_trim_material_id = Some(1)),
                )]),
                true,
            ),
            (
                bundle_patch(vec![template_stack(
                    0,
                    1,
                    patch_with(&[56], |p| p.armor_trim_material_id = Some(0)),
                )]),
                false,
            ),
        ],
    );
    h.assert_trim_cases(
        45,
        "component_condition_container_partial_trim",
        [
            (
                container_patch(
                    vec![
                        template_stack(
                            0,
                            1,
                            patch_with(&[56], |p| {
                                p.armor_trim_material_id = Some(1);
                                p.armor_trim_pattern_id = Some(0);
                            }),
                        ),
                        template_stack(
                            1,
                            1,
                            patch_with(&[56], |p| {
                                p.armor_trim_material_id = Some(0);
                                p.armor_trim_pattern_id = Some(0);
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
                            patch_with(&[56], |p| {
                                p.armor_trim_material_id = Some(1);
                                p.armor_trim_pattern_id = Some(0);
                            }),
                        ),
                        template_stack(
                            1,
                            1,
                            patch_with(&[56], |p| {
                                p.armor_trim_material_id = Some(1);
                                p.armor_trim_pattern_id = Some(1);
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
        46,
        "component_condition_jukebox_playable_song",
        [
            (DataComponentPatchSummary::default(), false),
            (patch_with(&[64], |p| p.jukebox_song_id = Some(1)), true),
            (patch_with(&[64], |p| p.jukebox_song_id = Some(0)), false),
            (patch_with(&[64], |_| {}), false),
        ],
    );
    h.assert_cases(
        47,
        "component_condition_bundle_partial_jukebox_playable",
        [
            (
                bundle_patch(vec![template_stack(
                    0,
                    1,
                    patch_with(&[64], |p| p.jukebox_song_id = Some(1)),
                )]),
                true,
            ),
            (
                bundle_patch(vec![template_stack(
                    0,
                    1,
                    patch_with(&[64], |p| p.jukebox_song_id = Some(0)),
                )]),
                false,
            ),
        ],
    );
    h.assert_cases(
        48,
        "component_condition_container_partial_jukebox_playable",
        [
            (
                container_patch(
                    vec![
                        template_stack(0, 1, patch_with(&[64], |p| p.jukebox_song_id = Some(1))),
                        template_stack(1, 1, patch_with(&[64], |p| p.jukebox_song_id = Some(1))),
                    ],
                    Some(2),
                ),
                true,
            ),
            (
                container_patch(
                    vec![
                        template_stack(0, 1, patch_with(&[64], |p| p.jukebox_song_id = Some(1))),
                        template_stack(1, 1, patch_with(&[64], |p| p.jukebox_song_id = Some(0))),
                    ],
                    Some(2),
                ),
                false,
            ),
        ],
    );
    h.assert_cases(
        49,
        "component_condition_potion_contents",
        [
            (DataComponentPatchSummary::default(), false),
            (
                patch_with(&[51], |p| {
                    p.potion_contents = bbb_protocol::packets::PotionContentsSummary {
                        potion_id: Some(healing_potion_id),
                        ..Default::default()
                    };
                }),
                true,
            ),
            (
                patch_with(&[51], |p| {
                    p.potion_contents = bbb_protocol::packets::PotionContentsSummary {
                        potion_id: Some(0),
                        ..Default::default()
                    };
                }),
                false,
            ),
            (patch_with(&[51], |_| {}), false),
        ],
    );
    h.assert_cases(
        50,
        "component_condition_bundle_partial_potion_contents",
        [
            (
                bundle_patch(vec![template_stack(
                    0,
                    1,
                    patch_with(&[51], |p| {
                        p.potion_contents = bbb_protocol::packets::PotionContentsSummary {
                            potion_id: Some(healing_potion_id),
                            ..Default::default()
                        };
                    }),
                )]),
                true,
            ),
            (
                bundle_patch(vec![template_stack(
                    0,
                    1,
                    patch_with(&[51], |p| {
                        p.potion_contents = bbb_protocol::packets::PotionContentsSummary {
                            potion_id: Some(0),
                            ..Default::default()
                        };
                    }),
                )]),
                false,
            ),
        ],
    );
    h.assert_cases(
        51,
        "component_condition_container_partial_potion_contents",
        [
            (
                container_patch(
                    vec![
                        template_stack(
                            0,
                            1,
                            patch_with(&[51], |p| {
                                p.potion_contents = bbb_protocol::packets::PotionContentsSummary {
                                    potion_id: Some(healing_potion_id),
                                    ..Default::default()
                                };
                            }),
                        ),
                        template_stack(
                            1,
                            1,
                            patch_with(&[51], |p| {
                                p.potion_contents = bbb_protocol::packets::PotionContentsSummary {
                                    potion_id: Some(healing_potion_id),
                                    ..Default::default()
                                };
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
                            patch_with(&[51], |p| {
                                p.potion_contents = bbb_protocol::packets::PotionContentsSummary {
                                    potion_id: Some(healing_potion_id),
                                    ..Default::default()
                                };
                            }),
                        ),
                        template_stack(
                            1,
                            1,
                            patch_with(&[51], |p| {
                                p.potion_contents = bbb_protocol::packets::PotionContentsSummary {
                                    potion_id: Some(0),
                                    ..Default::default()
                                };
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
        52,
        "component_condition_writable_book_pages",
        [
            (DataComponentPatchSummary::default(), false),
            (
                patch_with(&[54], |p| {
                    p.writable_book_pages = vec!["alpha".into(), "beta".into(), "alpha".into()]
                }),
                true,
            ),
            (
                patch_with(&[54], |p| {
                    p.writable_book_pages = vec!["alpha".into(), "beta".into()]
                }),
                false,
            ),
            (
                patch_with(&[54], |p| {
                    p.removed_type_ids = vec![54];
                    p.writable_book_pages = vec!["alpha".into(), "beta".into(), "alpha".into()];
                }),
                false,
            ),
        ],
    );

    let matching_written_book = WrittenBookContentSummary {
        title: "Quest".into(),
        title_filter: None,
        author: "Alex".into(),
        generation: 2,
        pages: vec!["First page".to_string(), "Second page".to_string()],
        page_filters: vec![None, None],
        resolved: true,
    };
    h.assert_cases(
        53,
        "component_condition_written_book_content",
        [
            (DataComponentPatchSummary::default(), false),
            (
                patch_with(&[55], |p| {
                    p.written_book = Some(matching_written_book.clone())
                }),
                true,
            ),
            (
                patch_with(&[55], |p| {
                    p.written_book = Some(WrittenBookContentSummary {
                        author: "Steve".into(),
                        ..matching_written_book.clone()
                    });
                }),
                false,
            ),
            (
                patch_with(&[55], |p| {
                    p.written_book = Some(WrittenBookContentSummary {
                        resolved: false,
                        ..matching_written_book.clone()
                    });
                }),
                false,
            ),
            (
                patch_with(&[55], |p| {
                    p.removed_type_ids = vec![55];
                    p.written_book = Some(matching_written_book.clone());
                }),
                false,
            ),
        ],
    );
    h.assert_cases(
        80,
        "component_condition_written_book_component_page",
        [
            (
                patch_with(&[55], |p| {
                    p.written_book = Some(matching_written_book.clone())
                }),
                true,
            ),
            (
                patch_with(&[55], |p| {
                    p.written_book = Some(WrittenBookContentSummary {
                        pages: vec!["Other page".to_string(), "Second page".to_string()],
                        ..matching_written_book.clone()
                    });
                }),
                false,
            ),
        ],
    );
    h.assert_cases(
        54,
        "component_condition_bundle_partial_writable_book",
        [
            (
                bundle_patch(vec![template_stack(
                    0,
                    1,
                    patch_with(&[54], |p| {
                        p.writable_book_pages = vec!["alpha".into(), "beta".into(), "alpha".into()]
                    }),
                )]),
                true,
            ),
            (
                bundle_patch(vec![template_stack(
                    0,
                    1,
                    patch_with(&[54], |p| {
                        p.writable_book_pages = vec!["alpha".into(), "beta".into()]
                    }),
                )]),
                false,
            ),
        ],
    );
    h.assert_cases(
        55,
        "component_condition_container_partial_written_book",
        [
            (
                container_patch(
                    vec![
                        template_stack(
                            0,
                            1,
                            patch_with(&[55], |p| {
                                p.written_book = Some(matching_written_book.clone())
                            }),
                        ),
                        template_stack(
                            1,
                            1,
                            patch_with(&[55], |p| {
                                p.written_book = Some(matching_written_book.clone())
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
                            patch_with(&[55], |p| p.written_book = Some(matching_written_book)),
                        ),
                        template_stack(
                            1,
                            1,
                            patch_with(&[55], |p| {
                                p.written_book = Some(WrittenBookContentSummary {
                                    title: "Other".into(),
                                    title_filter: None,
                                    author: "Alex".into(),
                                    generation: 2,
                                    pages: vec![
                                        "First page".to_string(),
                                        "Second page".to_string(),
                                    ],
                                    page_filters: vec![None, None],
                                    resolved: true,
                                });
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
        56,
        "component_condition_villager_variant",
        [
            (DataComponentPatchSummary::default(), false),
            (patch_with(&[81], |p| p.villager_variant_id = Some(2)), true),
            (
                patch_with(&[81], |p| p.villager_variant_id = Some(0)),
                false,
            ),
            (
                patch_with(&[81], |p| {
                    p.removed_type_ids = vec![81];
                    p.villager_variant_id = Some(2);
                }),
                false,
            ),
        ],
    );
    h.assert_cases(
        57,
        "component_condition_bundle_partial_villager_variant",
        [
            (
                bundle_patch(vec![template_stack(
                    0,
                    1,
                    patch_with(&[81], |p| p.villager_variant_id = Some(2)),
                )]),
                true,
            ),
            (
                bundle_patch(vec![template_stack(
                    0,
                    1,
                    patch_with(&[81], |p| p.villager_variant_id = Some(0)),
                )]),
                false,
            ),
        ],
    );
    h.assert_cases(
        58,
        "component_condition_container_partial_villager_variant",
        [
            (
                container_patch(
                    vec![
                        template_stack(
                            0,
                            1,
                            patch_with(&[81], |p| p.villager_variant_id = Some(2)),
                        ),
                        template_stack(
                            1,
                            1,
                            patch_with(&[81], |p| p.villager_variant_id = Some(0)),
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
                            patch_with(&[81], |p| p.villager_variant_id = Some(0)),
                        ),
                        template_stack(
                            1,
                            1,
                            patch_with(&[81], |p| p.villager_variant_id = Some(0)),
                        ),
                    ],
                    Some(2),
                ),
                false,
            ),
        ],
    );
    h.assert_cases(
        70,
        "component_condition_villager_variant_tag",
        [
            (patch_with(&[81], |p| p.villager_variant_id = Some(2)), true),
            (
                patch_with(&[81], |p| p.villager_variant_id = Some(0)),
                false,
            ),
        ],
    );
    h.assert_cases(
        71,
        "component_condition_bundle_partial_villager_variant_tag",
        [
            (
                bundle_patch(vec![template_stack(
                    0,
                    1,
                    patch_with(&[81], |p| p.villager_variant_id = Some(2)),
                )]),
                true,
            ),
            (
                bundle_patch(vec![template_stack(
                    0,
                    1,
                    patch_with(&[81], |p| p.villager_variant_id = Some(0)),
                )]),
                false,
            ),
        ],
    );
    h.assert_cases(
        72,
        "component_condition_container_partial_villager_variant_tag",
        [
            (
                container_patch(
                    vec![template_stack(
                        0,
                        1,
                        patch_with(&[81], |p| p.villager_variant_id = Some(2)),
                    )],
                    Some(1),
                ),
                true,
            ),
            (
                container_patch(
                    vec![template_stack(
                        0,
                        1,
                        patch_with(&[81], |p| p.villager_variant_id = Some(0)),
                    )],
                    Some(1),
                ),
                false,
            ),
        ],
    );

    let attribute_id_for_key = |resource_id: &str| {
        i32::try_from(
            h.attribute_keys
                .iter()
                .position(|candidate| candidate == resource_id)
                .expect("builtin attribute registry key"),
        )
        .unwrap()
    };
    let unknown_attribute_id = i32::try_from(h.attribute_keys.len()).unwrap();
    let modifier_with_attribute = |attribute_id, id: &str, amount: f64, slot_id| {
        AttributeModifierSummary {
            attribute_id,
            modifier_id: id.to_string(),
            amount_bits: amount.to_bits(),
            operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
            slot_id,
            display_id: 0,
            display_text: None,
            display_text_styled: None,
            display_text_nbt: None,
        }
    };
    let add_modifier = |id: &str, amount: f64, slot_id| {
        AttributeModifierSummary {
            attribute_id: unknown_attribute_id,
            modifier_id: id.to_string(),
            amount_bits: amount.to_bits(),
            operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
            slot_id,
            display_id: 0,
            display_text: None,
            display_text_styled: None,
            display_text_nbt: None,
        }
    };
    let multiplied_base_modifier = |id: &str, amount: f64, slot_id| {
        AttributeModifierSummary {
            attribute_id: unknown_attribute_id,
            modifier_id: id.to_string(),
            amount_bits: amount.to_bits(),
            operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
            slot_id,
            display_id: 0,
            display_text: None,
            display_text_styled: None,
            display_text_nbt: None,
        }
    };
    h.assert_cases(
        59,
        "component_condition_attribute_modifiers",
        [
            (DataComponentPatchSummary::default(), false),
            (
                patch_with(&[16], |p| {
                    p.attribute_modifiers = vec![
                        add_modifier("minecraft:test/speed", 1.5, 1),
                        multiplied_base_modifier("minecraft:test/scale", 3.0, 3),
                    ]
                }),
                true,
            ),
            (
                patch_with(&[16], |p| {
                    p.attribute_modifiers = vec![
                        add_modifier("minecraft:test/speed", 0.5, 1),
                        multiplied_base_modifier("minecraft:test/scale", 3.0, 3),
                    ]
                }),
                false,
            ),
            (
                patch_with(&[16], |p| {
                    p.removed_type_ids = vec![16];
                    p.attribute_modifiers = vec![
                        add_modifier("minecraft:test/speed", 1.5, 1),
                        multiplied_base_modifier("minecraft:test/scale", 3.0, 3),
                    ];
                }),
                false,
            ),
        ],
    );
    h.assert_cases(
        60,
        "component_condition_container_partial_attribute_modifiers",
        [
            (
                container_patch(
                    vec![template_stack(
                        0,
                        1,
                        patch_with(&[16], |p| {
                            p.attribute_modifiers = vec![
                                add_modifier("minecraft:test/speed", 1.5, 1),
                                multiplied_base_modifier("minecraft:test/scale", 3.0, 3),
                            ]
                        }),
                    )],
                    Some(1),
                ),
                true,
            ),
            (
                container_patch(
                    vec![template_stack(
                        0,
                        1,
                        patch_with(&[16], |p| {
                            p.attribute_modifiers = vec![
                                add_modifier("minecraft:test/speed", 1.5, 1),
                                add_modifier("minecraft:test/heavy", 2.0, 2),
                            ]
                        }),
                    )],
                    Some(1),
                ),
                false,
            ),
        ],
    );
    let attack_damage_modifier = modifier_with_attribute(
        attribute_id_for_key("minecraft:attack_damage"),
        "minecraft:test/speed",
        1.5,
        1,
    );
    let scale_modifier = modifier_with_attribute(
        attribute_id_for_key("minecraft:scale"),
        "minecraft:test/speed",
        1.5,
        1,
    );
    let attribute_patch = |modifier| patch_with(&[16], |p| p.attribute_modifiers = vec![modifier]);
    h.assert_cases(
        61,
        "component_condition_attribute_modifiers_attribute",
        [(attribute_patch(attack_damage_modifier.clone()), false)],
    );
    h.assert_attr_cases(
        61,
        "component_condition_attribute_modifiers_attribute",
        [
            (attribute_patch(attack_damage_modifier.clone()), true),
            (attribute_patch(scale_modifier.clone()), false),
        ],
    );
    let container_attribute_patch = |modifier| {
        container_patch(
            vec![template_stack(0, 1, attribute_patch(modifier))],
            Some(1),
        )
    };
    h.assert_attr_cases(
        62,
        "component_condition_container_partial_attribute_modifiers_attribute",
        [
            (
                container_attribute_patch(attack_damage_modifier.clone()),
                true,
            ),
            (container_attribute_patch(scale_modifier.clone()), false),
        ],
    );
    let bundle_attribute_modifiers_patch = |modifiers| {
        bundle_patch(vec![template_stack(
            0,
            1,
            patch_with(&[16], |p| p.attribute_modifiers = modifiers),
        )])
    };
    h.assert_cases(
        66,
        "component_condition_bundle_partial_attribute_modifiers",
        [
            (
                bundle_attribute_modifiers_patch(vec![
                    add_modifier("minecraft:test/speed", 1.5, 1),
                    multiplied_base_modifier("minecraft:test/scale", 3.0, 3),
                ]),
                true,
            ),
            (
                bundle_attribute_modifiers_patch(vec![
                    add_modifier("minecraft:test/speed", 1.5, 1),
                    add_modifier("minecraft:test/heavy", 2.0, 2),
                ]),
                false,
            ),
            (
                DataComponentPatchSummary {
                    removed_type_ids: vec![50],
                    ..bundle_attribute_modifiers_patch(vec![
                        add_modifier("minecraft:test/speed", 1.5, 1),
                        multiplied_base_modifier("minecraft:test/scale", 3.0, 3),
                    ])
                },
                false,
            ),
        ],
    );
    let bundle_attribute_patch =
        |modifier| bundle_patch(vec![template_stack(0, 1, attribute_patch(modifier))]);
    h.assert_attr_cases(
        67,
        "component_condition_bundle_partial_attribute_modifiers_attribute",
        [
            (bundle_attribute_patch(attack_damage_modifier.clone()), true),
            (bundle_attribute_patch(scale_modifier.clone()), false),
        ],
    );
    h.assert_attr_cases(
        68,
        "component_condition_attribute_modifiers_attribute_tag",
        [
            (attribute_patch(attack_damage_modifier.clone()), true),
            (attribute_patch(scale_modifier.clone()), false),
        ],
    );
    h.assert_attr_cases(
        69,
        "component_condition_bundle_partial_attribute_modifiers_attribute_tag",
        [
            (bundle_attribute_patch(attack_damage_modifier), true),
            (bundle_attribute_patch(scale_modifier), false),
        ],
    );
    h.assert_attr_cases(
        73,
        "component_condition_default_attribute_modifiers",
        [
            (DataComponentPatchSummary::default(), true),
            (patch_removed(&[16]), false),
            (
                patch_with(&[16], |p| p.attribute_modifiers = Vec::new()),
                false,
            ),
        ],
    );
    let default_attribute_item = |component_patch| ItemStackTemplateSummary {
        item_id: 73,
        count: 1,
        component_patch,
    };
    let bundle_default_attribute_patch =
        |component_patch| bundle_patch(vec![default_attribute_item(component_patch)]);
    h.assert_attr_cases(
        74,
        "component_condition_bundle_partial_default_attribute_modifiers",
        [
            (
                bundle_default_attribute_patch(DataComponentPatchSummary::default().into()),
                true,
            ),
            (
                bundle_default_attribute_patch(patch_removed(&[16]).into()),
                false,
            ),
        ],
    );
    let container_default_attribute_patch =
        |component_patch| container_patch(vec![default_attribute_item(component_patch)], Some(1));
    h.assert_attr_cases(
        75,
        "component_condition_container_partial_default_attribute_modifiers",
        [
            (
                container_default_attribute_patch(DataComponentPatchSummary::default().into()),
                true,
            ),
            (
                container_default_attribute_patch(patch_removed(&[16]).into()),
                false,
            ),
        ],
    );
    h.assert_attr_cases(
        76,
        "component_condition_default_armor_attribute_modifiers",
        [
            (DataComponentPatchSummary::default(), true),
            (patch_removed(&[16]), false),
        ],
    );
    h.assert_attr_cases(
        77,
        "component_condition_default_mace_attribute_modifiers",
        [
            (DataComponentPatchSummary::default(), true),
            (patch_removed(&[16]), false),
        ],
    );
    let custom_data_value = |owner: &str| {
        NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "owner".into(),
                value: NbtSummaryValue::string(owner),
            },
            NbtSummaryEntry {
                name: "level".into(),
                value: NbtSummaryValue::Int(7),
            },
            NbtSummaryEntry {
                name: "nested".into(),
                value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "flag".into(),
                    value: NbtSummaryValue::Byte(1),
                }]),
            },
            NbtSummaryEntry {
                name: "lore".into(),
                value: NbtSummaryValue::List(vec![
                    NbtSummaryValue::string("one"),
                    NbtSummaryValue::string("two"),
                ]),
            },
        ])
    };
    let custom_data_patch =
        |owner| patch_with(&[0], |p| p.custom_data = Some(custom_data_value(owner)));
    h.assert_cases(
        63,
        "component_condition_custom_data",
        [
            (DataComponentPatchSummary::default(), false),
            (custom_data_patch("Alex"), true),
            (custom_data_patch("Steve"), false),
            (
                DataComponentPatchSummary {
                    removed_type_ids: vec![0],
                    custom_data: Some(custom_data_value("Alex")),
                    ..DataComponentPatchSummary::default()
                },
                false,
            ),
        ],
    );
    let bundle_custom_data_patch =
        |owner| bundle_patch(vec![template_stack(0, 1, custom_data_patch(owner))]);
    h.assert_cases(
        64,
        "component_condition_bundle_partial_custom_data",
        [
            (bundle_custom_data_patch("Alex"), true),
            (bundle_custom_data_patch("Steve"), false),
            (
                DataComponentPatchSummary {
                    removed_type_ids: vec![50],
                    ..bundle_custom_data_patch("Alex")
                },
                false,
            ),
        ],
    );
    let container_custom_data_patch = |owner| {
        container_patch(
            vec![template_stack(0, 1, custom_data_patch(owner))],
            Some(1),
        )
    };
    h.assert_cases(
        65,
        "component_condition_container_partial_custom_data",
        [
            (container_custom_data_patch("Alex"), true),
            (container_custom_data_patch("Steve"), false),
        ],
    );
    h.assert_cases(
        78,
        "component_condition_custom_data_snbt",
        [
            (custom_data_patch("Alex"), true),
            (custom_data_patch("Steve"), false),
        ],
    );
    h.assert_cases(
        79,
        "component_condition_bundle_partial_custom_data_snbt",
        [
            (bundle_custom_data_patch("Alex"), true),
            (bundle_custom_data_patch("Steve"), false),
        ],
    );
}

#[test]
fn native_item_runtime_consumes_prepared_fonts_without_second_atlas_allocation() {
    let root = unique_temp_dir("item-runtime-prepared-fonts");
    write_component_condition_fixture(&root);
    let roots = PackRoots::from_root(&root).unwrap();
    let runtime = NativeItemRuntime::load_with_locale_and_fonts(&roots, "en_us", None).unwrap();
    assert!(runtime.map_text_fonts().is_none());
    let item_bytes = roots.resource_image_memory_observation().charged_rgba_bytes;
    drop(runtime);
    drop(roots);

    write_json(
        &assets_dir(&root).join("font/default.json"),
        r#"{"providers":[{"type":"space","advances":{"D":3}}]}"#,
    );
    let roots = PackRoots::from_root(&root).unwrap();
    let selected = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(
        &roots,
        std::iter::empty::<PathBuf>(),
    )
    .unwrap();
    let roots = roots
        .with_client_resource_generation_image_limit(1, selected, item_bytes + 5 * 8 * 4)
        .unwrap();
    let atlas = load_hud_font_atlas_with_options(&roots, FontOptions::default()).unwrap();
    let runtime =
        NativeItemRuntime::load_with_locale_and_fonts(&roots, "en_us", Some(atlas.fonts.clone()))
            .unwrap();
    assert_eq!(
        runtime
            .map_text_fonts()
            .unwrap()
            .glyphs(None)
            .resolve('D')
            .advance,
        3.0
    );
    assert_eq!(
        roots.resource_image_memory_observation().charged_rgba_bytes,
        item_bytes + 160
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_propagates_default_font_preparation_failure() {
    let root = unique_temp_dir("item-runtime-invalid-fonts");
    write_component_condition_fixture(&root);
    write_json(
        &assets_dir(&root).join("font/default.json"),
        r#"{"providers":[{"type":"bitmap","file":"font/absent.png","ascent":7,"chars":["X"]}]}"#,
    );
    let error = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap())
        .err()
        .unwrap();
    assert!(
        format!("{error:#}").contains("default font failed to load"),
        "{error:#}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn ordinary_cuboid_composite_keeps_each_resource_geometry_and_texture() {
    let root = unique_temp_dir("ordinary-cuboid-composite");
    let assets = assets_dir(&root);
    write_item_atlases(&assets);
    write_single_item_registry_fixture(&root, "test_cuboid");
    write_item_registry_override_with_profiles(
        &root,
        &["test_cuboid"],
        &[("test_cuboid", "test_cuboid")],
    );
    write_json(
        &assets.join("items/test_cuboid.json"),
        r#"{
        "model":{"type":"minecraft:composite","models":[
            {"type":"minecraft:model","model":"minecraft:item/cuboid_red"},
            {"type":"minecraft:model","model":"minecraft:item/cuboid_blue"}
        ]}}"#,
    );
    for (name, from, to, color) in [
        ("cuboid_red", [0, 0, 0], [16, 16, 2], [255, 0, 0, 255]),
        ("cuboid_blue", [4, 1, 3], [12, 7, 11], [0, 0, 255, 255]),
    ] {
        write_json(
            &assets.join(format!("models/item/{name}.json")),
            &format!(
                r##"{{
            "textures":{{"layer0":"minecraft:item/{name}"}},
            "elements":[{{"from":{from:?},"to":{to:?},"faces":{{
                "south":{{"texture":"#layer0"}},"north":{{"texture":"#layer0"}},
                "up":{{"texture":"#layer0"}},"down":{{"texture":"#layer0"}},
                "east":{{"texture":"#layer0"}},"west":{{"texture":"#layer0"}}
            }}}}]}}"##
            ),
        );
        write_test_rgba_png(
            &assets.join(format!("textures/item/{name}.png")),
            1,
            1,
            &color,
        );
    }
    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: Default::default(),
    };
    let resolved = runtime
        .resolve_item_model(
            &stack,
            &ItemModelContext::new(BlockModelDisplayContext::FirstPersonRightHand),
        )
        .unwrap();
    let [ResolvedItemModelLayer::Cuboid { parts: red, .. }, ResolvedItemModelLayer::Cuboid { parts: blue, .. }] =
        resolved.layers.as_slice()
    else {
        panic!("ordinary composite must preserve both cuboid leaves")
    };
    let red_quads = crate::ItemModelAssetSource::Model(red[0].geometry.clone()).build();
    let blue_quads = crate::ItemModelAssetSource::Model(blue[0].geometry.clone()).build();
    assert_eq!(red[0].model_id.as_ref(), "minecraft:item/cuboid_red");
    assert_eq!(blue[0].model_id.as_ref(), "minecraft:item/cuboid_blue");
    assert!(red_quads
        .iter()
        .flat_map(|q| q.corners)
        .any(|p| p == [16.0, 16.0, 2.0]));
    assert!(blue_quads
        .iter()
        .flat_map(|q| q.corners)
        .any(|p| p == [12.0, 7.0, 11.0]));
    assert_ne!(red_quads[0].uvs, blue_quads[0].uvs);
    std::fs::remove_dir_all(root).unwrap();
}
