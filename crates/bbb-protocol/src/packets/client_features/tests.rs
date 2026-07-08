use super::*;
use crate::{
    codec::{Decoder, Encoder},
    ids,
    packets::{
        decode_play_clientbound, DataComponentPatchSummary, ItemStackSummary, PlayClientbound,
    },
};

#[test]
fn decodes_custom_chat_completions_packet_wire_order() {
    let mut payload = Encoder::new();
    payload.write_var_i32(2);
    payload.write_var_i32(2);
    payload.write_string("/warp");
    payload.write_string("/spawn");
    let payload = payload.into_inner();

    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_CUSTOM_CHAT_COMPLETIONS, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::CustomChatCompletions(CustomChatCompletions {
            action: CustomChatCompletionsAction::Set,
            entries: vec!["/warp".to_string(), "/spawn".to_string()],
        })
    );

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 2);
    assert_eq!(decoder.read_var_i32().unwrap(), 2);
    assert_eq!(decoder.read_string(32767).unwrap(), "/warp");
    assert_eq!(decoder.read_string(32767).unwrap(), "/spawn");
    assert!(decoder.is_empty());
}

#[test]
fn rejects_unknown_custom_chat_completion_action() {
    let mut payload = Encoder::new();
    payload.write_var_i32(3);
    payload.write_var_i32(0);

    let err = decode_play_clientbound(
        ids::play::CLIENTBOUND_CUSTOM_CHAT_COMPLETIONS,
        &payload.into_inner(),
    )
    .unwrap_err();
    assert!(err
        .to_string()
        .contains("invalid custom chat completions action ordinal 3"));
}

#[test]
fn decodes_place_ghost_recipe_packet_wire_prefix() {
    let mut payload = Encoder::new();
    payload.write_var_i32(9);
    payload.write_var_i32(3);
    payload.write_var_i32(4);
    payload.write_var_i32(100);
    payload.write_var_i32(4);
    payload.write_var_i32(101);
    payload.write_var_i32(4);
    payload.write_var_i32(102);
    let payload = payload.into_inner();

    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_PLACE_GHOST_RECIPE, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::PlaceGhostRecipe(PlaceGhostRecipe {
            container_id: 9,
            recipe_display: RecipeDisplaySummary {
                display_type: RecipeDisplayType::Stonecutter,
                raw_body: vec![3, 4, 100, 4, 101, 4, 102],
                crafting: None,
                furnace: None,
            },
        })
    );

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 9);
    assert_eq!(decoder.read_var_i32().unwrap(), 3);
    assert_eq!(decoder.read_var_i32().unwrap(), 4);
    assert_eq!(decoder.read_var_i32().unwrap(), 100);
    assert_eq!(decoder.read_var_i32().unwrap(), 4);
    assert_eq!(decoder.read_var_i32().unwrap(), 101);
    assert_eq!(decoder.read_var_i32().unwrap(), 4);
    assert_eq!(decoder.read_var_i32().unwrap(), 102);
    assert!(decoder.is_empty());
}

#[test]
fn rejects_unknown_recipe_display_type() {
    let mut payload = Encoder::new();
    payload.write_var_i32(9);
    payload.write_var_i32(5);

    let err = decode_play_clientbound(
        ids::play::CLIENTBOUND_PLACE_GHOST_RECIPE,
        &payload.into_inner(),
    )
    .unwrap_err();
    assert!(err.to_string().contains("invalid recipe display type id 5"));
}

#[test]
fn decodes_recipe_book_add_packet_wire_order() {
    let mut payload = Encoder::new();
    payload.write_var_i32(1);
    payload.write_var_i32(123);
    payload.write_var_i32(3);
    payload.write_var_i32(0);
    payload.write_var_i32(0);
    payload.write_var_i32(0);
    payload.write_var_i32(8);
    payload.write_var_i32(10);
    payload.write_bool(true);
    payload.write_var_i32(1);
    payload.write_var_i32(3);
    payload.write_var_i32(42);
    payload.write_var_i32(43);
    payload.write_u8(3);
    payload.write_bool(true);
    let payload = payload.into_inner();

    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_RECIPE_BOOK_ADD, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::RecipeBookAdd(RecipeBookAdd {
            entries: vec![RecipeBookAddEntry {
                contents: RecipeDisplayEntry {
                    id: RecipeDisplayId { index: 123 },
                    display: RecipeDisplaySummary {
                        display_type: RecipeDisplayType::Stonecutter,
                        raw_body: vec![3, 0, 0, 0],
                        crafting: None,
                        furnace: None,
                    },
                    group: Some(7),
                    category_id: 10,
                    crafting_requirements: Some(vec![IngredientSummary {
                        tag: None,
                        item_ids: vec![42, 43],
                    }]),
                },
                flags: 3,
                notification: true,
                highlight: true,
            }],
            replace: true,
        })
    );
}

#[test]
fn decodes_recipe_book_add_with_structured_shaped_crafting_display() {
    let mut payload = Encoder::new();
    payload.write_var_i32(1);
    payload.write_var_i32(200);
    payload.write_var_i32(1);
    payload.write_var_i32(2);
    payload.write_var_i32(1);
    payload.write_var_i32(2);
    payload.write_var_i32(4);
    payload.write_var_i32(42);
    payload.write_var_i32(5);
    payload.write_var_i32(43);
    payload.write_var_i32(1);
    payload.write_var_i32(0);
    payload.write_var_i32(0);
    payload.write_var_i32(5);
    payload.write_var_i32(90);
    payload.write_var_i32(2);
    payload.write_var_i32(0);
    payload.write_var_i32(0);
    payload.write_var_i32(0);
    payload.write_var_i32(0);
    payload.write_var_i32(10);
    payload.write_bool(false);
    payload.write_u8(0);
    payload.write_bool(false);

    let PlayClientbound::RecipeBookAdd(packet) = decode_play_clientbound(
        ids::play::CLIENTBOUND_RECIPE_BOOK_ADD,
        &payload.into_inner(),
    )
    .unwrap() else {
        panic!("expected recipe book add packet");
    };
    let entry = &packet.entries[0].contents;
    assert_eq!(
        entry.display.display_type,
        RecipeDisplayType::CraftingShaped
    );
    assert_eq!(
        entry.display.crafting,
        Some(CraftingRecipeDisplaySummary::Shaped {
            width: 2,
            height: 1,
            ingredients: vec![
                SlotDisplaySummary {
                    display_type_id: 4,
                    raw_payload: vec![4, 42],
                    item_stack: Some(item_stack(42, 1)),
                    tag: None,
                },
                SlotDisplaySummary {
                    display_type_id: 5,
                    raw_payload: vec![5, 43, 1, 0, 0],
                    item_stack: Some(item_stack(43, 1)),
                    tag: None,
                },
            ],
            result: SlotDisplaySummary {
                display_type_id: 5,
                raw_payload: vec![5, 90, 2, 0, 0],
                item_stack: Some(item_stack(90, 2)),
                tag: None,
            },
            crafting_station: SlotDisplaySummary {
                display_type_id: 0,
                raw_payload: vec![0],
                item_stack: None,
                tag: None,
            },
        })
    );
}

#[test]
fn decodes_recipe_book_add_with_structured_shapeless_crafting_display() {
    let mut payload = Encoder::new();
    payload.write_var_i32(1);
    payload.write_var_i32(201);
    payload.write_var_i32(0);
    payload.write_var_i32(2);
    payload.write_var_i32(4);
    payload.write_var_i32(42);
    payload.write_var_i32(4);
    payload.write_var_i32(43);
    payload.write_var_i32(4);
    payload.write_var_i32(91);
    payload.write_var_i32(0);
    payload.write_var_i32(0);
    payload.write_var_i32(10);
    payload.write_bool(false);
    payload.write_u8(0);
    payload.write_bool(false);

    let PlayClientbound::RecipeBookAdd(packet) = decode_play_clientbound(
        ids::play::CLIENTBOUND_RECIPE_BOOK_ADD,
        &payload.into_inner(),
    )
    .unwrap() else {
        panic!("expected recipe book add packet");
    };
    let entry = &packet.entries[0].contents;
    assert_eq!(
        entry.display.crafting,
        Some(CraftingRecipeDisplaySummary::Shapeless {
            ingredients: vec![
                SlotDisplaySummary {
                    display_type_id: 4,
                    raw_payload: vec![4, 42],
                    item_stack: Some(item_stack(42, 1)),
                    tag: None,
                },
                SlotDisplaySummary {
                    display_type_id: 4,
                    raw_payload: vec![4, 43],
                    item_stack: Some(item_stack(43, 1)),
                    tag: None,
                },
            ],
            result: SlotDisplaySummary {
                display_type_id: 4,
                raw_payload: vec![4, 91],
                item_stack: Some(item_stack(91, 1)),
                tag: None,
            },
            crafting_station: SlotDisplaySummary {
                display_type_id: 0,
                raw_payload: vec![0],
                item_stack: None,
                tag: None,
            },
        })
    );
}

#[test]
fn decodes_recipe_book_add_with_structured_furnace_display() {
    let mut payload = Encoder::new();
    payload.write_var_i32(1);
    payload.write_var_i32(202);
    payload.write_var_i32(2);
    payload.write_var_i32(4);
    payload.write_var_i32(42);
    payload.write_var_i32(1);
    payload.write_var_i32(5);
    payload.write_var_i32(90);
    payload.write_var_i32(2);
    payload.write_var_i32(0);
    payload.write_var_i32(0);
    payload.write_var_i32(0);
    payload.write_var_i32(200);
    payload.write_f32(0.35);
    payload.write_var_i32(0);
    payload.write_var_i32(4);
    payload.write_bool(false);
    payload.write_u8(0);
    payload.write_bool(false);

    let PlayClientbound::RecipeBookAdd(packet) = decode_play_clientbound(
        ids::play::CLIENTBOUND_RECIPE_BOOK_ADD,
        &payload.into_inner(),
    )
    .unwrap() else {
        panic!("expected recipe book add packet");
    };
    let entry = &packet.entries[0].contents;
    assert_eq!(entry.display.display_type, RecipeDisplayType::Furnace);
    assert_eq!(entry.display.crafting, None);
    assert_eq!(
        entry.display.furnace,
        Some(FurnaceRecipeDisplaySummary {
            ingredient: SlotDisplaySummary {
                display_type_id: 4,
                raw_payload: vec![4, 42],
                item_stack: Some(item_stack(42, 1)),
                tag: None,
            },
            fuel: SlotDisplaySummary {
                display_type_id: 1,
                raw_payload: vec![1],
                item_stack: None,
                tag: None,
            },
            result: SlotDisplaySummary {
                display_type_id: 5,
                raw_payload: vec![5, 90, 2, 0, 0],
                item_stack: Some(item_stack(90, 2)),
                tag: None,
            },
            crafting_station: SlotDisplaySummary {
                display_type_id: 0,
                raw_payload: vec![0],
                item_stack: None,
                tag: None,
            },
            duration: 200,
            experience_bits: 0.35_f32.to_bits(),
        })
    );
}

#[test]
fn decodes_recipe_book_remove_packet_wire_order() {
    let mut payload = Encoder::new();
    payload.write_var_i32(2);
    payload.write_var_i32(123);
    payload.write_var_i32(124);

    assert_eq!(
        decode_play_clientbound(
            ids::play::CLIENTBOUND_RECIPE_BOOK_REMOVE,
            &payload.into_inner()
        )
        .unwrap(),
        PlayClientbound::RecipeBookRemove(RecipeBookRemove {
            recipe_ids: vec![
                RecipeDisplayId { index: 123 },
                RecipeDisplayId { index: 124 },
            ],
        })
    );
}

#[test]
fn decodes_recipe_book_settings_packet_wire_order() {
    let mut payload = Encoder::new();
    payload.write_bool(true);
    payload.write_bool(false);
    payload.write_bool(false);
    payload.write_bool(true);
    payload.write_bool(true);
    payload.write_bool(true);
    payload.write_bool(false);
    payload.write_bool(false);

    assert_eq!(
        decode_play_clientbound(
            ids::play::CLIENTBOUND_RECIPE_BOOK_SETTINGS,
            &payload.into_inner()
        )
        .unwrap(),
        PlayClientbound::RecipeBookSettings(RecipeBookSettings {
            crafting: RecipeBookTypeSettings {
                open: true,
                filtering: false,
            },
            furnace: RecipeBookTypeSettings {
                open: false,
                filtering: true,
            },
            blast_furnace: RecipeBookTypeSettings {
                open: true,
                filtering: true,
            },
            smoker: RecipeBookTypeSettings {
                open: false,
                filtering: false,
            },
        })
    );
}

#[test]
fn decodes_update_recipes_packet_wire_order() {
    let mut payload = Encoder::new();
    payload.write_var_i32(2);
    payload.write_string("furnace_input");
    payload.write_var_i32(2);
    payload.write_var_i32(42);
    payload.write_var_i32(43);
    payload.write_string("minecraft:smithing_base");
    payload.write_var_i32(1);
    payload.write_var_i32(99);

    payload.write_var_i32(1);
    payload.write_var_i32(0);
    payload.write_string("planks");
    payload.write_var_i32(4);
    payload.write_var_i32(77);

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_UPDATE_RECIPES, &payload.into_inner())
            .unwrap(),
        PlayClientbound::UpdateRecipes(UpdateRecipes {
            property_sets: vec![
                RecipePropertySetSummary {
                    key: "minecraft:furnace_input".to_string(),
                    item_ids: vec![42, 43],
                },
                RecipePropertySetSummary {
                    key: "minecraft:smithing_base".to_string(),
                    item_ids: vec![99],
                },
            ],
            stonecutter_recipes: vec![StonecutterSelectableRecipeSummary {
                input: IngredientSummary {
                    tag: Some("minecraft:planks".to_string()),
                    item_ids: Vec::new(),
                },
                option_display: SlotDisplaySummary {
                    display_type_id: 4,
                    raw_payload: vec![4, 77],
                    item_stack: Some(item_stack(77, 1)),
                    tag: None,
                },
            }],
        })
    );
}

#[test]
fn decodes_update_recipes_with_direct_trim_pattern_slot_display() {
    let mut option_display = Encoder::new();
    option_display.write_var_i32(8);
    option_display.write_var_i32(0);
    option_display.write_var_i32(0);
    option_display.write_var_i32(0);
    option_display.write_string("test_trim");
    option_display.write_bytes(&nbt_string_root("Test Trim"));
    option_display.write_bool(true);
    let option_display = option_display.into_inner();

    let mut payload = Encoder::new();
    payload.write_var_i32(0);
    payload.write_var_i32(1);
    payload.write_var_i32(2);
    payload.write_var_i32(11);
    payload.write_bytes(&option_display);

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_UPDATE_RECIPES, &payload.into_inner())
            .unwrap(),
        PlayClientbound::UpdateRecipes(UpdateRecipes {
            property_sets: Vec::new(),
            stonecutter_recipes: vec![StonecutterSelectableRecipeSummary {
                input: IngredientSummary {
                    tag: None,
                    item_ids: vec![11],
                },
                option_display: SlotDisplaySummary {
                    display_type_id: 8,
                    raw_payload: option_display,
                    item_stack: None,
                    tag: None,
                },
            }],
        })
    );
}

#[test]
fn decodes_update_recipes_with_tag_slot_display() {
    let mut option_display = Encoder::new();
    option_display.write_var_i32(6);
    option_display.write_string("planks");
    let option_display = option_display.into_inner();

    let mut payload = Encoder::new();
    payload.write_var_i32(0);
    payload.write_var_i32(1);
    payload.write_var_i32(2);
    payload.write_var_i32(11);
    payload.write_bytes(&option_display);

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_UPDATE_RECIPES, &payload.into_inner())
            .unwrap(),
        PlayClientbound::UpdateRecipes(UpdateRecipes {
            property_sets: Vec::new(),
            stonecutter_recipes: vec![StonecutterSelectableRecipeSummary {
                input: IngredientSummary {
                    tag: None,
                    item_ids: vec![11],
                },
                option_display: SlotDisplaySummary {
                    display_type_id: 6,
                    raw_payload: option_display,
                    item_stack: None,
                    tag: Some("minecraft:planks".to_string()),
                },
            }],
        })
    );
}

#[test]
fn slot_display_summary_exposes_composite_stack_resolving_children() {
    let item_display = slot_display_item_payload(77);
    let tag_display = slot_display_tag_payload("planks");
    let mut option_display = Encoder::new();
    option_display.write_var_i32(10);
    option_display.write_var_i32(2);
    option_display.write_bytes(&item_display);
    option_display.write_bytes(&tag_display);

    let option_display = decode_update_recipes_option_display(option_display.into_inner());
    assert_eq!(
        option_display.stack_resolving_children(),
        vec![
            SlotDisplaySummary {
                display_type_id: 4,
                raw_payload: item_display,
                item_stack: Some(item_stack(77, 1)),
                tag: None,
            },
            SlotDisplaySummary {
                display_type_id: 6,
                raw_payload: tag_display,
                item_stack: None,
                tag: Some("minecraft:planks".to_string()),
            },
        ]
    );
}

#[test]
fn slot_display_summary_exposes_with_remainder_input_for_stack_resolution() {
    let input_display = slot_display_item_payload(55);
    let remainder_display = slot_display_item_stack_payload(56, 1);
    let mut option_display = Encoder::new();
    option_display.write_var_i32(9);
    option_display.write_bytes(&input_display);
    option_display.write_bytes(&remainder_display);

    let option_display = decode_update_recipes_option_display(option_display.into_inner());
    assert_eq!(
        option_display.stack_resolving_children(),
        vec![SlotDisplaySummary {
            display_type_id: 4,
            raw_payload: input_display,
            item_stack: Some(item_stack(55, 1)),
            tag: None,
        }]
    );
}

#[test]
fn rejects_invalid_update_recipes_identifiers() {
    let mut invalid_property_key = Encoder::new();
    invalid_property_key.write_var_i32(1);
    invalid_property_key.write_string("minecraft:FurnaceInput");
    let err = decode_play_clientbound(
        ids::play::CLIENTBOUND_UPDATE_RECIPES,
        &invalid_property_key.into_inner(),
    )
    .unwrap_err();
    assert!(err.to_string().contains("invalid resource location"));

    let mut invalid_ingredient_tag = Encoder::new();
    invalid_ingredient_tag.write_var_i32(0);
    invalid_ingredient_tag.write_var_i32(1);
    invalid_ingredient_tag.write_var_i32(0);
    invalid_ingredient_tag.write_string("minecraft:Planks");
    let err = decode_play_clientbound(
        ids::play::CLIENTBOUND_UPDATE_RECIPES,
        &invalid_ingredient_tag.into_inner(),
    )
    .unwrap_err();
    assert!(err.to_string().contains("invalid resource location"));

    let mut invalid_slot_tag = Encoder::new();
    invalid_slot_tag.write_var_i32(0);
    invalid_slot_tag.write_var_i32(1);
    invalid_slot_tag.write_var_i32(2);
    invalid_slot_tag.write_var_i32(11);
    invalid_slot_tag.write_var_i32(6);
    invalid_slot_tag.write_string("minecraft:Planks");
    let err = decode_play_clientbound(
        ids::play::CLIENTBOUND_UPDATE_RECIPES,
        &invalid_slot_tag.into_inner(),
    )
    .unwrap_err();
    assert!(err.to_string().contains("invalid resource location"));

    let mut invalid_trim = Encoder::new();
    invalid_trim.write_var_i32(0);
    invalid_trim.write_var_i32(1);
    invalid_trim.write_var_i32(2);
    invalid_trim.write_var_i32(11);
    invalid_trim.write_var_i32(8);
    invalid_trim.write_var_i32(0);
    invalid_trim.write_var_i32(0);
    invalid_trim.write_var_i32(0);
    invalid_trim.write_string("minecraft:TestTrim");
    let err = decode_play_clientbound(
        ids::play::CLIENTBOUND_UPDATE_RECIPES,
        &invalid_trim.into_inner(),
    )
    .unwrap_err();
    assert!(err.to_string().contains("invalid resource location"));
}

#[test]
fn decodes_select_advancements_tab_present_and_absent() {
    let mut present = Encoder::new();
    present.write_bool(true);
    present.write_string("minecraft:story/root");
    let present = present.into_inner();

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_SELECT_ADVANCEMENTS_TAB, &present).unwrap(),
        PlayClientbound::SelectAdvancementsTab(SelectAdvancementsTab {
            tab: Some("minecraft:story/root".to_string()),
        })
    );

    let mut absent = Encoder::new();
    absent.write_bool(false);
    let absent = absent.into_inner();

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_SELECT_ADVANCEMENTS_TAB, &absent).unwrap(),
        PlayClientbound::SelectAdvancementsTab(SelectAdvancementsTab { tab: None })
    );
}

#[test]
fn normalizes_select_advancements_tab_default_namespace() {
    let mut payload = Encoder::new();
    payload.write_bool(true);
    payload.write_string("story/root");
    let payload = payload.into_inner();

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_SELECT_ADVANCEMENTS_TAB, &payload).unwrap(),
        PlayClientbound::SelectAdvancementsTab(SelectAdvancementsTab {
            tab: Some("minecraft:story/root".to_string()),
        })
    );
}

#[test]
fn rejects_invalid_select_advancements_tab_identifier() {
    for tab in ["minecraft:Story/root", "minecraft:story root"] {
        let mut payload = Encoder::new();
        payload.write_bool(true);
        payload.write_string(tab);
        let payload = payload.into_inner();

        let err = decode_play_clientbound(ids::play::CLIENTBOUND_SELECT_ADVANCEMENTS_TAB, &payload)
            .unwrap_err();
        assert!(
            err.to_string().contains("invalid resource location"),
            "unexpected error for {tab}: {err}"
        );
    }
}

#[test]
fn decodes_update_advancements_packet_wire_order() {
    let mut payload = Encoder::new();
    payload.write_bool(true);

    payload.write_var_i32(1);
    payload.write_string("minecraft:story/root");
    payload.write_bool(false);
    payload.write_bool(true);
    payload.write_bytes(&nbt_string_root("Root"));
    payload.write_bytes(&nbt_string_root("Description"));
    payload.write_var_i32(42);
    payload.write_var_i32(1);
    payload.write_var_i32(0);
    payload.write_var_i32(0);
    payload.write_var_i32(0);
    payload.write_i32(3);
    payload.write_string("minecraft:textures/gui/advancements/backgrounds/stone.png");
    payload.write_f32(1.5);
    payload.write_f32(-2.0);
    payload.write_var_i32(1);
    payload.write_var_i32(2);
    payload.write_string("mine_stone");
    payload.write_string("get_log");
    payload.write_bool(true);

    payload.write_var_i32(1);
    payload.write_string("minecraft:old");

    payload.write_var_i32(1);
    payload.write_string("minecraft:story/root");
    payload.write_var_i32(2);
    payload.write_string("mine_stone");
    payload.write_bool(true);
    payload.write_i64(1_700_000_000_000);
    payload.write_string("get_log");
    payload.write_bool(false);

    payload.write_bool(true);

    assert_eq!(
        decode_play_clientbound(
            ids::play::CLIENTBOUND_UPDATE_ADVANCEMENTS,
            &payload.into_inner()
        )
        .unwrap(),
        PlayClientbound::UpdateAdvancements(UpdateAdvancements {
            reset: true,
            added: vec![AdvancementSummary {
                id: "minecraft:story/root".to_string(),
                parent: None,
                display: Some(AdvancementDisplaySummary {
                    title: "Root".to_string(),
                    description: "Description".to_string(),
                    icon: AdvancementIconSummary {
                        item_id: 42,
                        count: 1,
                        component_patch: DataComponentPatchSummary::default(),
                    },
                    frame_type: AdvancementFrameType::Task,
                    show_toast: true,
                    hidden: false,
                    background: Some(
                        "minecraft:textures/gui/advancements/backgrounds/stone.png".to_string()
                    ),
                    x: 1.5,
                    y: -2.0,
                }),
                requirements: vec![vec!["mine_stone".to_string(), "get_log".to_string()]],
                sends_telemetry_event: true,
            }],
            removed: vec!["minecraft:old".to_string()],
            progress: vec![AdvancementProgressSummary {
                id: "minecraft:story/root".to_string(),
                criteria: vec![
                    AdvancementCriterionProgressSummary {
                        name: "mine_stone".to_string(),
                        obtained_epoch_millis: Some(1_700_000_000_000),
                    },
                    AdvancementCriterionProgressSummary {
                        name: "get_log".to_string(),
                        obtained_epoch_millis: None,
                    },
                ],
            }],
            show_advancements: true,
        })
    );
}

#[test]
fn normalizes_update_advancements_identifiers() {
    let mut payload = Encoder::new();
    payload.write_bool(false);

    payload.write_var_i32(1);
    payload.write_string("story/root");
    payload.write_bool(true);
    payload.write_string("story/parent");
    payload.write_bool(true);
    payload.write_bytes(&nbt_string_root("Root"));
    payload.write_bytes(&nbt_string_root("Description"));
    payload.write_var_i32(42);
    payload.write_var_i32(1);
    payload.write_var_i32(0);
    payload.write_var_i32(0);
    payload.write_var_i32(0);
    payload.write_i32(1);
    payload.write_string("textures/gui/advancements/backgrounds/stone.png");
    payload.write_f32(0.0);
    payload.write_f32(0.0);
    payload.write_var_i32(1);
    payload.write_var_i32(1);
    payload.write_string("mine_stone");
    payload.write_bool(false);

    payload.write_var_i32(1);
    payload.write_string("old");

    payload.write_var_i32(1);
    payload.write_string("story/root");
    payload.write_var_i32(1);
    payload.write_string("mine_stone");
    payload.write_bool(false);

    payload.write_bool(false);

    assert_eq!(
        decode_play_clientbound(
            ids::play::CLIENTBOUND_UPDATE_ADVANCEMENTS,
            &payload.into_inner()
        )
        .unwrap(),
        PlayClientbound::UpdateAdvancements(UpdateAdvancements {
            reset: false,
            added: vec![AdvancementSummary {
                id: "minecraft:story/root".to_string(),
                parent: Some("minecraft:story/parent".to_string()),
                display: Some(AdvancementDisplaySummary {
                    title: "Root".to_string(),
                    description: "Description".to_string(),
                    icon: AdvancementIconSummary {
                        item_id: 42,
                        count: 1,
                        component_patch: DataComponentPatchSummary::default(),
                    },
                    frame_type: AdvancementFrameType::Task,
                    show_toast: false,
                    hidden: false,
                    background: Some(
                        "minecraft:textures/gui/advancements/backgrounds/stone.png".to_string()
                    ),
                    x: 0.0,
                    y: 0.0,
                }),
                requirements: vec![vec!["mine_stone".to_string()]],
                sends_telemetry_event: false,
            }],
            removed: vec!["minecraft:old".to_string()],
            progress: vec![AdvancementProgressSummary {
                id: "minecraft:story/root".to_string(),
                criteria: vec![AdvancementCriterionProgressSummary {
                    name: "mine_stone".to_string(),
                    obtained_epoch_millis: None,
                }],
            }],
            show_advancements: false,
        })
    );
}

#[test]
fn rejects_invalid_update_advancements_identifier() {
    let mut payload = Encoder::new();
    payload.write_bool(false);
    payload.write_var_i32(1);
    payload.write_string("minecraft:Story/root");
    let payload = payload.into_inner();

    let err =
        decode_play_clientbound(ids::play::CLIENTBOUND_UPDATE_ADVANCEMENTS, &payload).unwrap_err();
    assert!(err.to_string().contains("invalid resource location"));
}

#[test]
fn decodes_tag_query_packet_raw_nbt() {
    let mut payload = Encoder::new();
    payload.write_var_i32(12);
    payload.write_bytes(&compound_with_string("name", "Chest"));
    let payload = payload.into_inner();

    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_TAG_QUERY, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::TagQuery(TagQuery {
            transaction_id: 12,
            tag_present: true,
            raw_nbt: compound_with_string("name", "Chest"),
        })
    );

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 12);
    assert_eq!(
        decoder
            .read_exact(decoder.remaining_len(), "tag query nbt")
            .unwrap(),
        compound_with_string("name", "Chest").as_slice()
    );
    assert!(decoder.is_empty());
}

#[test]
fn decodes_null_tag_query_packet() {
    let mut payload = Encoder::new();
    payload.write_var_i32(13);
    payload.write_u8(0);

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_TAG_QUERY, &payload.into_inner()).unwrap(),
        PlayClientbound::TagQuery(TagQuery {
            transaction_id: 13,
            tag_present: false,
            raw_nbt: vec![0],
        })
    );
}

#[test]
fn rejects_null_tag_query_with_trailing_bytes() {
    let mut payload = Encoder::new();
    payload.write_var_i32(13);
    payload.write_u8(0);
    payload.write_u8(1);

    let err = decode_play_clientbound(ids::play::CLIENTBOUND_TAG_QUERY, &payload.into_inner())
        .unwrap_err();
    assert!(err
        .to_string()
        .contains("trailing bytes after null tag query nbt"));
}

fn item_stack(item_id: i32, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: DataComponentPatchSummary::default(),
    }
}

fn decode_update_recipes_option_display(option_display: Vec<u8>) -> SlotDisplaySummary {
    let mut payload = Encoder::new();
    payload.write_var_i32(0);
    payload.write_var_i32(1);
    payload.write_var_i32(2);
    payload.write_var_i32(11);
    payload.write_bytes(&option_display);
    let PlayClientbound::UpdateRecipes(UpdateRecipes {
        mut stonecutter_recipes,
        ..
    }) = decode_play_clientbound(ids::play::CLIENTBOUND_UPDATE_RECIPES, &payload.into_inner())
        .unwrap()
    else {
        panic!("expected update recipes packet");
    };
    stonecutter_recipes.remove(0).option_display
}

fn slot_display_item_payload(item_id: i32) -> Vec<u8> {
    let mut payload = Encoder::new();
    payload.write_var_i32(4);
    payload.write_var_i32(item_id);
    payload.into_inner()
}

fn slot_display_item_stack_payload(item_id: i32, count: i32) -> Vec<u8> {
    let mut payload = Encoder::new();
    payload.write_var_i32(5);
    payload.write_var_i32(item_id);
    payload.write_var_i32(count);
    payload.write_var_i32(0);
    payload.write_var_i32(0);
    payload.into_inner()
}

fn slot_display_tag_payload(tag: &str) -> Vec<u8> {
    let mut payload = Encoder::new();
    payload.write_var_i32(6);
    payload.write_string(tag);
    payload.into_inner()
}

fn compound_with_string(name: &str, value: &str) -> Vec<u8> {
    let mut payload = vec![10, 8];
    write_mutf8(&mut payload, name);
    write_mutf8(&mut payload, value);
    payload.push(0);
    payload
}

fn nbt_string_root(value: &str) -> Vec<u8> {
    let mut payload = vec![8];
    write_mutf8(&mut payload, value);
    payload
}

fn write_mutf8(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u16).to_be_bytes());
    out.extend_from_slice(value.as_bytes());
}
