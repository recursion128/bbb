use super::*;

#[test]
fn native_item_runtime_shows_spawner_block_entity_tooltips() {
    let root = unique_temp_dir("item-runtime-spawner-tooltip");
    let assets = assets_dir(&root);
    write_item_atlases(&assets);
    write_item_registry_fixture(&root, &["spawner", "trial_spawner"]);
    write_json(
        &assets.join("lang").join("en_us.json"),
        r#"{
                "block.minecraft.spawner": "Monster Spawner",
                "block.minecraft.trial_spawner": "Trial Spawner",
                "block.minecraft.spawner.desc1": "Interact with Spawn Egg:",
                "block.minecraft.spawner.desc2": "Sets Mob Type",
                "entity.minecraft.zombie": "Zombie"
            }"#,
    );

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();

    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        }),
        Some(vec![
            name_line("Monster Spawner", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("", TOOLTIP_TEXT_WHITE),
            tooltip_line("Interact with Spawn Egg:", TOOLTIP_TEXT_GRAY),
            tooltip_line(" Sets Mob Type", TOOLTIP_TEXT_BLUE),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                block_entity_spawn_entity_type: Some("minecraft:zombie".to_string()),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Monster Spawner", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Zombie", TOOLTIP_TEXT_GRAY),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                tooltip_hidden_component_type_ids: vec![60],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![name_line(
            "Monster Spawner",
            TOOLTIP_TEXT_WHITE,
            0xFF_FF_FF,
            false
        )])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(1),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        }),
        Some(vec![
            name_line("Trial Spawner", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("", TOOLTIP_TEXT_WHITE),
            tooltip_line("Interact with Spawn Egg:", TOOLTIP_TEXT_GRAY),
            tooltip_line(" Sets Mob Type", TOOLTIP_TEXT_BLUE),
        ])
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_elides_all_brick_pot_decoration_tooltips() {
    let root = unique_temp_dir("item-runtime-pot-all-brick-tooltip");
    let assets = assets_dir(&root);
    write_item_atlases(&assets);
    write_item_registry_fixture(&root, &["decorated_pot", "brick", "test_sherd"]);
    write_json(
        &assets.join("lang").join("en_us.json"),
        r#"{
                "item.minecraft.decorated_pot": "Decorated Pot",
                "item.minecraft.brick": "Brick",
                "item.minecraft.test_sherd": "Test Sherd"
            }"#,
    );

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();

    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                pot_decorations_item_ids: vec![1, 1, 1, 1],
                lore: vec!["After all-brick pot".to_string()],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Decorated Pot", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            lore_line("After all-brick pot"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                pot_decorations_item_ids: vec![1, 1, 1, 2],
                lore: vec!["After sherd pot".to_string()],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Decorated Pot", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("", TOOLTIP_TEXT_WHITE),
            tooltip_line("Test Sherd", TOOLTIP_TEXT_GRAY),
            tooltip_line("Brick", TOOLTIP_TEXT_GRAY),
            tooltip_line("Brick", TOOLTIP_TEXT_GRAY),
            tooltip_line("Brick", TOOLTIP_TEXT_GRAY),
            lore_line("After sherd pot"),
        ])
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_shows_entity_data_peaceful_warning() {
    let root = unique_temp_dir("item-runtime-entity-data-tooltip");
    let assets = assets_dir(&root);
    write_item_atlases(&assets);
    write_item_registry_fixture(&root, &["zombie_spawn_egg", "pig_spawn_egg"]);
    write_json(
        &assets.join("lang").join("en_us.json"),
        r#"{
                "item.minecraft.zombie_spawn_egg": "Zombie Spawn Egg",
                "item.minecraft.pig_spawn_egg": "Pig Spawn Egg",
                "item.spawn_egg.peaceful": "This creature cannot spawn in Peaceful mode"
            }"#,
    );

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let zombie_stack = ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary {
            entity_data_entity_type_id: Some(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            ),
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };

    assert_eq!(
        runtime.tooltip_lines_for_stack(&zombie_stack),
        Some(vec![name_line(
            "Zombie Spawn Egg",
            TOOLTIP_TEXT_WHITE,
            0xFF_FF_FF,
            false
        )])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(
            &zombie_stack,
            NativeItemTooltipOptions {
                peaceful: true,
                ..NativeItemTooltipOptions::default()
            },
        ),
        Some(vec![
            name_line("Zombie Spawn Egg", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line(
                "This creature cannot spawn in Peaceful mode",
                TOOLTIP_TEXT_RED
            ),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(
            &ItemStackSummary {
                component_patch: DataComponentPatchSummary {
                    tooltip_hidden_component_type_ids: vec![58],
                    ..zombie_stack.component_patch.clone().into_inner()
                }
                .into(),
                ..zombie_stack.clone()
            },
            NativeItemTooltipOptions {
                peaceful: true,
                ..NativeItemTooltipOptions::default()
            },
        ),
        Some(vec![name_line(
            "Zombie Spawn Egg",
            TOOLTIP_TEXT_WHITE,
            0xFF_FF_FF,
            false
        )])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(
            &ItemStackSummary {
                item_id: Some(1),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    entity_data_entity_type_id: Some(
                        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID,
                    ),
                    ..DataComponentPatchSummary::default()
                }
                .into(),
            },
            NativeItemTooltipOptions {
                peaceful: true,
                ..NativeItemTooltipOptions::default()
            },
        ),
        Some(vec![name_line(
            "Pig Spawn Egg",
            TOOLTIP_TEXT_WHITE,
            0xFF_FF_FF,
            false
        )])
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_shows_operator_only_nbt_warnings() {
    let root = unique_temp_dir("item-runtime-op-nbt-warning");
    let assets = assets_dir(&root);
    write_item_atlases(&assets);
    write_item_registry_fixture(
        &root,
        &[
            "command_block",
            "stone",
            "falling_block_spawn_egg",
            "zombie_spawn_egg",
            "stick",
        ],
    );
    write_item_registry_override_with_profiles(
        &root,
        &[
            "command_block",
            "stone",
            "falling_block_spawn_egg",
            "zombie_spawn_egg",
            "stick",
        ],
        &[
            ("command_block", "command_block"),
            ("stone", "stone"),
            ("falling_block_spawn_egg", "zombie_spawn_egg"),
            ("zombie_spawn_egg", "zombie_spawn_egg"),
        ],
    );
    write_json(
        &assets.join("lang").join("en_us.json"),
        r#"{
                "block.minecraft.command_block": "Command Block",
                "block.minecraft.stone": "Stone",
                "item.minecraft.falling_block_spawn_egg": "Falling Block Spawn Egg",
                "item.minecraft.zombie_spawn_egg": "Zombie Spawn Egg",
                "item.minecraft.stick": "Stick",
                "item.op_warning.line1": "Operator data",
                "item.op_warning.line2": "Can alter commands",
                "item.op_warning.line3": "Use with care"
            }"#,
    );
    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let warnings = vec![
        operator_warning_line("Operator data", true),
        operator_warning_line("Can alter commands", false),
        operator_warning_line("Use with care", false),
    ];
    let op_options = NativeItemTooltipOptions {
        gamemaster_permission: true,
        ..NativeItemTooltipOptions::default()
    };
    let command_block = ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary {
            block_entity_data_type_id: Some(VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID),
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    let mut named_warnings = vec![name_line(
        "Command Block",
        TOOLTIP_TEXT_WHITE,
        0xFF_FF_FF,
        false,
    )];
    named_warnings.extend(warnings.clone());
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(&command_block, op_options),
        Some(named_warnings)
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&command_block),
        Some(vec![name_line(
            "Command Block",
            TOOLTIP_TEXT_WHITE,
            0xFF_FF_FF,
            false,
        )])
    );

    let falling_block_egg = ItemStackSummary {
        item_id: Some(2),
        count: 1,
        component_patch: DataComponentPatchSummary {
            entity_data_entity_type_id: Some(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
            ),
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    let mut egg_warnings = vec![name_line(
        "Falling Block Spawn Egg",
        TOOLTIP_TEXT_WHITE,
        0xFF_FF_FF,
        false,
    )];
    egg_warnings.extend(warnings.clone());
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(&falling_block_egg, op_options),
        Some(egg_warnings)
    );

    for stack in [
        ItemStackSummary {
            item_id: Some(1),
            count: 1,
            component_patch: DataComponentPatchSummary {
                block_entity_data_type_id: Some(VANILLA_BLOCK_ENTITY_TYPE_FURNACE_ID),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
        ItemStackSummary {
            item_id: Some(3),
            count: 1,
            component_patch: DataComponentPatchSummary {
                entity_data_entity_type_id: Some(
                    bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                ),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
        ItemStackSummary {
            item_id: Some(4),
            count: 1,
            component_patch: command_block.component_patch.clone(),
        },
        ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                removed_type_ids: vec![60],
                ..command_block.component_patch.clone().into_inner()
            }
            .into(),
        },
    ] {
        assert_eq!(
            runtime
                .tooltip_lines_for_stack_with_context(&stack, op_options)
                .unwrap()
                .len(),
            1
        );
    }

    let hidden_command_block = ItemStackSummary {
        component_patch: DataComponentPatchSummary {
            tooltip_hide_tooltip: true,
            ..command_block.component_patch.clone().into_inner()
        }
        .into(),
        ..command_block
    };
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(&hidden_command_block, op_options),
        Some(warnings)
    );
    assert_eq!(runtime.tooltip_lines_for_stack(&hidden_command_block), None);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_shows_adventure_mode_predicate_tooltips() {
    let root = unique_temp_dir("item-runtime-adventure-mode-tooltip");
    let assets = assets_dir(&root);
    write_item_atlases(&assets);
    write_item_registry_fixture(&root, &["diamond_pickaxe"]);
    write_json(
        &assets.join("lang").join("en_us.json"),
        r#"{
                "item.minecraft.diamond_pickaxe": "Diamond Pickaxe",
                "item.canBreak": "Can break:",
                "item.canPlace": "Can be placed on:",
                "item.canUse.unknown": "Unknown",
                "block.minecraft.stone": "Stone",
                "block.minecraft.obsidian": "Obsidian"
            }"#,
    );

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let can_break = bbb_protocol::packets::AdventureModePredicateDataSummary {
        predicates: vec![bbb_protocol::packets::BlockPredicateSummary {
            blocks: Some(bbb_protocol::packets::HolderSetSummary {
                registry_ids: vec![1, 193, 9999, 1],
                tag: Some("minecraft:mineable/pickaxe".to_string()),
            }),
            ..Default::default()
        }],
    };
    let can_place_on = bbb_protocol::packets::AdventureModePredicateDataSummary {
        predicates: vec![bbb_protocol::packets::BlockPredicateSummary::default()],
    };

    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                can_break_persistent: Some(can_break.clone()),
                can_place_on_persistent: Some(can_place_on.clone()),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Diamond Pickaxe", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("", TOOLTIP_TEXT_WHITE),
            tooltip_line("Can break:", TOOLTIP_TEXT_GRAY),
            tooltip_line("Stone", TOOLTIP_TEXT_DARK_GRAY),
            tooltip_line("Obsidian", TOOLTIP_TEXT_DARK_GRAY),
            tooltip_line("", TOOLTIP_TEXT_WHITE),
            tooltip_line("Can be placed on:", TOOLTIP_TEXT_GRAY),
            tooltip_line("Unknown", TOOLTIP_TEXT_GRAY),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                can_break_persistent: Some(can_break),
                can_place_on_persistent: Some(can_place_on),
                tooltip_hidden_component_type_ids: vec![15],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Diamond Pickaxe", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("", TOOLTIP_TEXT_WHITE),
            tooltip_line("Can be placed on:", TOOLTIP_TEXT_GRAY),
            tooltip_line("Unknown", TOOLTIP_TEXT_GRAY),
        ])
    );

    std::fs::remove_dir_all(root).unwrap();
}
