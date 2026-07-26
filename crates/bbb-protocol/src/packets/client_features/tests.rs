use super::recipes::{
    MAX_RECIPE_BOOK_NESTED_LIST, MAX_RECIPE_DISPLAY_BODY, MAX_SLOT_DISPLAY_DEPTH,
};
use super::*;
use crate::{
    codec::{Decoder, Encoder},
    ids,
    packets::{
        decode_play_clientbound, DataComponentPatchSummary, ItemStackSummary, PlayClientbound,
    },
    recipe_wire_types::{
        VANILLA_RECIPE_BOOK_CATEGORY_STONECUTTER_ID, VANILLA_SLOT_DISPLAY_TYPE_ANY_FUEL_ID,
        VANILLA_SLOT_DISPLAY_TYPE_COMPOSITE_ID, VANILLA_SLOT_DISPLAY_TYPE_DYED_ID,
        VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID, VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,
        VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID, VANILLA_SLOT_DISPLAY_TYPE_ONLY_WITH_COMPONENT_ID,
        VANILLA_SLOT_DISPLAY_TYPE_SMITHING_TRIM_ID, VANILLA_SLOT_DISPLAY_TYPE_TAG_ID,
        VANILLA_SLOT_DISPLAY_TYPE_WITH_ANY_POTION_ID, VANILLA_SLOT_DISPLAY_TYPE_WITH_REMAINDER_ID,
    },
    RichTextComponent,
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

    let mut trailing = payload.clone();
    trailing.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_CUSTOM_CHAT_COMPLETIONS, &trailing)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after custom chat completions packet")
    );
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
                stonecutter: Some(StonecutterRecipeDisplaySummary {
                    input: item_slot_display_summary(100),
                    result: item_slot_display_summary(101),
                    crafting_station: item_slot_display_summary(102),
                }),
                smithing: None,
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

    let mut trailing = payload;
    trailing.push(0);
    let err =
        decode_play_clientbound(ids::play::CLIENTBOUND_PLACE_GHOST_RECIPE, &trailing).unwrap_err();
    assert!(err
        .to_string()
        .contains("trailing bytes after place ghost recipe packet"));
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
fn rejects_shaped_recipe_display_with_mismatched_dimensions() {
    let mut payload = Encoder::new();
    payload.write_var_i32(9);
    payload.write_var_i32(RecipeDisplayType::CraftingShaped.id());
    payload.write_var_i32(2);
    payload.write_var_i32(2);
    payload.write_var_i32(3);
    for _ in 0..3 {
        payload.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID);
    }
    payload.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID);
    payload.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID);

    let err = decode_play_clientbound(
        ids::play::CLIENTBOUND_PLACE_GHOST_RECIPE,
        &payload.into_inner(),
    )
    .unwrap_err();
    assert!(err
        .to_string()
        .contains("shaped recipe display has 3 ingredients for 2x2"));
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
                        stonecutter: Some(StonecutterRecipeDisplaySummary {
                            input: empty_slot_display_summary(),
                            result: empty_slot_display_summary(),
                            crafting_station: empty_slot_display_summary(),
                        }),
                        smithing: None,
                    },
                    group: Some(7),
                    category_id: VANILLA_RECIPE_BOOK_CATEGORY_STONECUTTER_ID,
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
    let mut trailing = payload;
    trailing.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_RECIPE_BOOK_ADD, &trailing)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after recipe book add packet")
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
                    display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,
                    raw_payload: vec![4, 42],
                    item_stack: Some(item_stack(42, 1)),
                    tag: None,
                },
                SlotDisplaySummary {
                    display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID,
                    raw_payload: vec![5, 43, 1, 0, 0],
                    item_stack: Some(item_stack(43, 1)),
                    tag: None,
                },
            ],
            result: SlotDisplaySummary {
                display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID,
                raw_payload: vec![5, 90, 2, 0, 0],
                item_stack: Some(item_stack(90, 2)),
                tag: None,
            },
            crafting_station: SlotDisplaySummary {
                display_type_id: VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
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
                    display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,
                    raw_payload: vec![4, 42],
                    item_stack: Some(item_stack(42, 1)),
                    tag: None,
                },
                SlotDisplaySummary {
                    display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,
                    raw_payload: vec![4, 43],
                    item_stack: Some(item_stack(43, 1)),
                    tag: None,
                },
            ],
            result: SlotDisplaySummary {
                display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,
                raw_payload: vec![4, 91],
                item_stack: Some(item_stack(91, 1)),
                tag: None,
            },
            crafting_station: SlotDisplaySummary {
                display_type_id: VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
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
                display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,
                raw_payload: vec![4, 42],
                item_stack: Some(item_stack(42, 1)),
                tag: None,
            },
            fuel: SlotDisplaySummary {
                display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ANY_FUEL_ID,
                raw_payload: vec![1],
                item_stack: None,
                tag: None,
            },
            result: SlotDisplaySummary {
                display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID,
                raw_payload: vec![5, 90, 2, 0, 0],
                item_stack: Some(item_stack(90, 2)),
                tag: None,
            },
            crafting_station: SlotDisplaySummary {
                display_type_id: VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
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
    let payload = payload.into_inner();

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_RECIPE_BOOK_REMOVE, &payload).unwrap(),
        PlayClientbound::RecipeBookRemove(RecipeBookRemove {
            recipe_ids: vec![
                RecipeDisplayId { index: 123 },
                RecipeDisplayId { index: 124 },
            ],
        })
    );
    let mut trailing = payload;
    trailing.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_RECIPE_BOOK_REMOVE, &trailing)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after recipe book remove packet")
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
    let payload = payload.into_inner();

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_RECIPE_BOOK_SETTINGS, &payload).unwrap(),
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
    let mut trailing = payload;
    trailing.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_RECIPE_BOOK_SETTINGS, &trailing)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after recipe book settings packet")
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

    let mut bytes = payload.into_inner();
    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_UPDATE_RECIPES, &bytes).unwrap(),
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
                    display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,
                    raw_payload: vec![4, 77],
                    item_stack: Some(item_stack(77, 1)),
                    tag: None,
                },
            }],
        })
    );
    bytes.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_UPDATE_RECIPES, &bytes)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after update recipes packet")
    );
}

#[test]
fn update_recipes_rejects_out_of_range_builtin_item_holders() {
    let invalid_item_id = crate::item_types::VANILLA_ITEM_TYPES.len() as i32;

    let mut property_set = Encoder::new();
    property_set.write_var_i32(1);
    property_set.write_string("minecraft:furnace_input");
    property_set.write_var_i32(1);
    property_set.write_var_i32(invalid_item_id);
    assert!(decode_play_clientbound(
        ids::play::CLIENTBOUND_UPDATE_RECIPES,
        &property_set.into_inner(),
    )
    .unwrap_err()
    .to_string()
    .contains(&format!("invalid vanilla item holder id {invalid_item_id}")));

    let mut item_display = Encoder::new();
    item_display.write_var_i32(0);
    item_display.write_var_i32(1);
    item_display.write_var_i32(0);
    item_display.write_string("minecraft:planks");
    item_display.write_var_i32(4);
    item_display.write_var_i32(invalid_item_id);
    assert!(decode_play_clientbound(
        ids::play::CLIENTBOUND_UPDATE_RECIPES,
        &item_display.into_inner(),
    )
    .unwrap_err()
    .to_string()
    .contains(&format!("invalid vanilla item holder id {invalid_item_id}")));

    for invalid_ingredient in [invalid_item_id, crate::item_types::VANILLA_ITEM_AIR_ID] {
        let mut ingredient = Encoder::new();
        ingredient.write_var_i32(0);
        ingredient.write_var_i32(1);
        ingredient.write_var_i32(2);
        ingredient.write_var_i32(invalid_ingredient);
        assert!(decode_play_clientbound(
            ids::play::CLIENTBOUND_UPDATE_RECIPES,
            &ingredient.into_inner(),
        )
        .unwrap_err()
        .to_string()
        .contains("item"));
    }

    let mut empty_ingredient = Encoder::new();
    empty_ingredient.write_var_i32(0);
    empty_ingredient.write_var_i32(1);
    empty_ingredient.write_var_i32(1);
    assert!(decode_play_clientbound(
        ids::play::CLIENTBOUND_UPDATE_RECIPES,
        &empty_ingredient.into_inner(),
    )
    .unwrap_err()
    .to_string()
    .contains("ingredient holder set must not be empty"));
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
                    display_type_id: VANILLA_SLOT_DISPLAY_TYPE_SMITHING_TRIM_ID,
                    raw_payload: option_display.clone(),
                    item_stack: None,
                    tag: None,
                },
            }],
        })
    );
    let SlotDisplayValue::SmithingTrim { pattern, .. } =
        decode_update_recipes_option_display(option_display)
            .decode_value()
            .unwrap()
    else {
        panic!("expected typed smithing trim display");
    };
    let direct = pattern.direct.expect("direct trim pattern");
    assert_eq!(pattern.registry_id, None);
    assert_eq!(direct.asset_id, "minecraft:test_trim");
    assert_eq!(direct.description, "Test Trim");
    assert!(direct.description_styled.is_some());
    assert!(direct.description_nbt.is_some());
    assert!(direct.decal);
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
                    display_type_id: VANILLA_SLOT_DISPLAY_TYPE_TAG_ID,
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
            SlotDisplayValue::Item {
                stack: Box::new(item_stack(77, 1)),
            },
            SlotDisplayValue::Tag {
                tag: "minecraft:planks".to_string(),
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
        vec![SlotDisplayValue::Item {
            stack: Box::new(item_stack(55, 1)),
        }]
    );
}

#[test]
fn decodes_all_builtin_slot_display_values_and_preserves_exact_raw_payloads() {
    let empty = slot_display_type_payload(VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID);
    let any_fuel = slot_display_type_payload(VANILLA_SLOT_DISPLAY_TYPE_ANY_FUEL_ID);
    let item = slot_display_item_payload(55);
    let item_stack_payload = slot_display_item_stack_payload(56, 2);
    let tag = slot_display_tag_payload("planks");
    let with_any_potion =
        slot_display_unary_payload(VANILLA_SLOT_DISPLAY_TYPE_WITH_ANY_POTION_ID, &item);
    let only_with_component = slot_display_with_component_payload(&item_stack_payload, 0);
    let dyed = slot_display_binary_payload(
        VANILLA_SLOT_DISPLAY_TYPE_DYED_ID,
        &item,
        &item_stack_payload,
    );
    let smithing_trim = slot_display_smithing_trim_payload(&item, &item_stack_payload, 3);
    let with_remainder = slot_display_binary_payload(
        VANILLA_SLOT_DISPLAY_TYPE_WITH_REMAINDER_ID,
        &item,
        &item_stack_payload,
    );
    let composite = slot_display_composite_payload(&[item.clone(), tag.clone()]);

    let cases = vec![
        (empty, SlotDisplayValue::Empty),
        (any_fuel, SlotDisplayValue::AnyFuel),
        (
            with_any_potion,
            SlotDisplayValue::WithAnyPotion {
                display: Box::new(SlotDisplayValue::Item {
                    stack: Box::new(item_stack(55, 1)),
                }),
            },
        ),
        (
            only_with_component,
            SlotDisplayValue::OnlyWithComponent {
                source: Box::new(SlotDisplayValue::ItemStack {
                    stack: Box::new(item_stack(56, 2)),
                }),
                component_type_id: 0,
            },
        ),
        (
            item,
            SlotDisplayValue::Item {
                stack: Box::new(item_stack(55, 1)),
            },
        ),
        (
            item_stack_payload,
            SlotDisplayValue::ItemStack {
                stack: Box::new(item_stack(56, 2)),
            },
        ),
        (
            tag,
            SlotDisplayValue::Tag {
                tag: "minecraft:planks".to_string(),
            },
        ),
        (
            dyed,
            SlotDisplayValue::Dyed {
                dye: Box::new(SlotDisplayValue::Item {
                    stack: Box::new(item_stack(55, 1)),
                }),
                target: Box::new(SlotDisplayValue::ItemStack {
                    stack: Box::new(item_stack(56, 2)),
                }),
            },
        ),
        (
            smithing_trim,
            SlotDisplayValue::SmithingTrim {
                base: Box::new(SlotDisplayValue::Item {
                    stack: Box::new(item_stack(55, 1)),
                }),
                material: Box::new(SlotDisplayValue::ItemStack {
                    stack: Box::new(item_stack(56, 2)),
                }),
                pattern: SlotDisplayTrimPatternSummary {
                    registry_id: Some(2),
                    direct: None,
                },
            },
        ),
        (
            with_remainder,
            SlotDisplayValue::WithRemainder {
                input: Box::new(SlotDisplayValue::Item {
                    stack: Box::new(item_stack(55, 1)),
                }),
                remainder: Box::new(SlotDisplayValue::ItemStack {
                    stack: Box::new(item_stack(56, 2)),
                }),
            },
        ),
        (
            composite,
            SlotDisplayValue::Composite {
                contents: vec![
                    SlotDisplayValue::Item {
                        stack: Box::new(item_stack(55, 1)),
                    },
                    SlotDisplayValue::Tag {
                        tag: "minecraft:planks".to_string(),
                    },
                ],
            },
        ),
    ];

    for (raw_payload, expected) in cases {
        let summary = decode_update_recipes_option_display(raw_payload.clone());
        assert_eq!(summary.raw_payload, raw_payload);
        assert_eq!(summary.decode_value().unwrap(), expected);
    }
}

#[test]
fn place_ghost_recipe_decodes_all_recipe_and_slot_display_variants() {
    let empty = slot_display_type_payload(VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID);
    let any_fuel = slot_display_type_payload(VANILLA_SLOT_DISPLAY_TYPE_ANY_FUEL_ID);
    let item = slot_display_item_payload(55);
    let item_stack_payload = slot_display_item_stack_payload(56, 2);
    let tag = slot_display_tag_payload("planks");
    let all_slot_displays = vec![
        empty.clone(),
        any_fuel,
        slot_display_unary_payload(VANILLA_SLOT_DISPLAY_TYPE_WITH_ANY_POTION_ID, &item),
        slot_display_with_component_payload(&item_stack_payload, 0),
        item.clone(),
        item_stack_payload.clone(),
        tag.clone(),
        slot_display_binary_payload(
            VANILLA_SLOT_DISPLAY_TYPE_DYED_ID,
            &item,
            &item_stack_payload,
        ),
        slot_display_smithing_trim_payload(&item, &item_stack_payload, 3),
        slot_display_binary_payload(
            VANILLA_SLOT_DISPLAY_TYPE_WITH_REMAINDER_ID,
            &item,
            &item_stack_payload,
        ),
        slot_display_composite_payload(&[item.clone(), tag]),
    ];

    let mut shapeless = Encoder::new();
    shapeless.write_var_i32(RecipeDisplayType::CraftingShapeless.id());
    shapeless.write_var_i32(all_slot_displays.len() as i32);
    for display in &all_slot_displays {
        shapeless.write_bytes(display);
    }
    shapeless.write_bytes(&slot_display_item_payload(60));
    shapeless.write_bytes(&empty);
    let shapeless = shapeless.into_inner();

    let mut shaped = Encoder::new();
    shaped.write_var_i32(RecipeDisplayType::CraftingShaped.id());
    shaped.write_var_i32(1);
    shaped.write_var_i32(1);
    shaped.write_var_i32(1);
    shaped.write_bytes(&item);
    shaped.write_bytes(&slot_display_item_payload(61));
    shaped.write_bytes(&empty);
    let shaped = shaped.into_inner();

    let mut furnace = Encoder::new();
    furnace.write_var_i32(RecipeDisplayType::Furnace.id());
    furnace.write_bytes(&item);
    furnace.write_bytes(&slot_display_type_payload(
        VANILLA_SLOT_DISPLAY_TYPE_ANY_FUEL_ID,
    ));
    furnace.write_bytes(&slot_display_item_payload(62));
    furnace.write_bytes(&empty);
    furnace.write_var_i32(200);
    furnace.write_f32(0.35);
    let furnace = furnace.into_inner();

    let mut stonecutter = Encoder::new();
    stonecutter.write_var_i32(RecipeDisplayType::Stonecutter.id());
    stonecutter.write_bytes(&item);
    stonecutter.write_bytes(&slot_display_item_payload(63));
    stonecutter.write_bytes(&empty);
    let stonecutter = stonecutter.into_inner();

    let mut smithing = Encoder::new();
    smithing.write_var_i32(RecipeDisplayType::Smithing.id());
    smithing.write_bytes(&item);
    smithing.write_bytes(&item_stack_payload);
    smithing.write_bytes(&slot_display_item_payload(57));
    smithing.write_bytes(&slot_display_item_payload(64));
    smithing.write_bytes(&empty);
    let smithing = smithing.into_inner();

    let cases = [
        (
            RecipeDisplayType::CraftingShapeless,
            shapeless.as_slice(),
            13,
            60,
        ),
        (RecipeDisplayType::CraftingShaped, shaped.as_slice(), 3, 61),
        (RecipeDisplayType::Furnace, furnace.as_slice(), 4, 62),
        (
            RecipeDisplayType::Stonecutter,
            stonecutter.as_slice(),
            3,
            63,
        ),
        (RecipeDisplayType::Smithing, smithing.as_slice(), 5, 64),
    ];

    for (display_type, raw_body, slot_count, result_item_id) in cases {
        let display = decode_place_ghost_recipe_display(raw_body);
        assert_eq!(display.display_type, display_type);
        assert_eq!(display.raw_body, raw_body);
        assert_eq!(display.slot_displays().unwrap().len(), slot_count);
        assert_eq!(
            display
                .result()
                .and_then(|result| result.item_stack.as_ref()),
            Some(&item_stack(result_item_id, 1))
        );
        assert_eq!(
            (
                display.crafting.is_some(),
                display.furnace.is_some(),
                display.stonecutter.is_some(),
                display.smithing.is_some(),
            ),
            match display_type {
                RecipeDisplayType::CraftingShapeless | RecipeDisplayType::CraftingShaped => {
                    (true, false, false, false)
                }
                RecipeDisplayType::Furnace => (false, true, false, false),
                RecipeDisplayType::Stonecutter => (false, false, true, false),
                RecipeDisplayType::Smithing => (false, false, false, true),
            }
        );
    }

    let display = decode_place_ghost_recipe_display(&shapeless);
    assert_eq!(
        display
            .slot_displays()
            .unwrap()
            .into_iter()
            .take(all_slot_displays.len())
            .map(|display| display.display_type_id)
            .collect::<Vec<_>>(),
        vec![
            VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
            VANILLA_SLOT_DISPLAY_TYPE_ANY_FUEL_ID,
            VANILLA_SLOT_DISPLAY_TYPE_WITH_ANY_POTION_ID,
            VANILLA_SLOT_DISPLAY_TYPE_ONLY_WITH_COMPONENT_ID,
            VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,
            VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID,
            VANILLA_SLOT_DISPLAY_TYPE_TAG_ID,
            VANILLA_SLOT_DISPLAY_TYPE_DYED_ID,
            VANILLA_SLOT_DISPLAY_TYPE_SMITHING_TRIM_ID,
            VANILLA_SLOT_DISPLAY_TYPE_WITH_REMAINDER_ID,
            VANILLA_SLOT_DISPLAY_TYPE_COMPOSITE_ID,
        ]
    );
}

#[test]
fn slot_display_decode_enforces_recursive_depth_count_and_byte_budgets() {
    let mut within_depth = slot_display_type_payload(VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID);
    for _ in 0..MAX_SLOT_DISPLAY_DEPTH {
        within_depth =
            slot_display_unary_payload(VANILLA_SLOT_DISPLAY_TYPE_WITH_ANY_POTION_ID, &within_depth);
    }
    assert!(decode_update_recipes_option_display(within_depth)
        .decode_value()
        .is_ok());

    let mut over_depth = slot_display_type_payload(VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID);
    for _ in 0..=MAX_SLOT_DISPLAY_DEPTH {
        over_depth =
            slot_display_unary_payload(VANILLA_SLOT_DISPLAY_TYPE_WITH_ANY_POTION_ID, &over_depth);
    }
    let err = decode_play_clientbound(
        ids::play::CLIENTBOUND_UPDATE_RECIPES,
        &update_recipes_payload(&over_depth),
    )
    .unwrap_err();
    assert!(err
        .to_string()
        .contains("slot display tree exceeded max depth"));

    let mut over_count = Encoder::new();
    over_count.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_COMPOSITE_ID);
    over_count.write_var_i32(MAX_RECIPE_BOOK_NESTED_LIST as i32 + 1);
    assert!(matches!(
        decode_play_clientbound(
            ids::play::CLIENTBOUND_UPDATE_RECIPES,
            &update_recipes_payload(&over_count.into_inner()),
        ),
        Err(crate::codec::ProtocolError::PacketTooLarge(
            actual,
            MAX_RECIPE_BOOK_NESTED_LIST
        )) if actual == MAX_RECIPE_BOOK_NESTED_LIST + 1
    ));

    let long_tag = "a".repeat(32_767);
    let tag_payload = slot_display_tag_payload(&long_tag);
    let oversized = slot_display_composite_payload(&vec![tag_payload; 65]);
    assert!(oversized.len() > MAX_RECIPE_DISPLAY_BODY);
    assert!(matches!(
        decode_play_clientbound(
            ids::play::CLIENTBOUND_UPDATE_RECIPES,
            &update_recipes_payload(&oversized),
        ),
        Err(crate::codec::ProtocolError::PacketTooLarge(
            _,
            MAX_RECIPE_DISPLAY_BODY
        ))
    ));
}

#[test]
fn slot_display_decode_rejects_invalid_types_holders_components_and_summaries() {
    let invalid_component = slot_display_with_component_payload(
        &slot_display_type_payload(VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID),
        crate::data_component_types::VANILLA_DATA_COMPONENT_TYPES.len() as i32,
    );
    let err = decode_play_clientbound(
        ids::play::CLIENTBOUND_UPDATE_RECIPES,
        &update_recipes_payload(&invalid_component),
    )
    .unwrap_err();
    assert!(err
        .to_string()
        .contains("invalid vanilla data component type id"));

    let err = decode_play_clientbound(
        ids::play::CLIENTBOUND_UPDATE_RECIPES,
        &update_recipes_payload(&slot_display_type_payload(11)),
    )
    .unwrap_err();
    assert!(err.to_string().contains("invalid slot display type id 11"));

    let invalid_holder = slot_display_smithing_trim_payload(
        &slot_display_type_payload(VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID),
        &slot_display_type_payload(VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID),
        -1,
    );
    assert!(matches!(
        decode_play_clientbound(
            ids::play::CLIENTBOUND_UPDATE_RECIPES,
            &update_recipes_payload(&invalid_holder),
        ),
        Err(crate::codec::ProtocolError::NegativeLength(-1))
    ));

    let item = slot_display_item_payload(55);
    let mismatched = SlotDisplaySummary {
        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_TAG_ID,
        raw_payload: item.clone(),
        item_stack: None,
        tag: None,
    };
    assert!(mismatched
        .decode_value()
        .unwrap_err()
        .to_string()
        .contains("does not match raw type"));

    let mut trailing = item;
    trailing.push(0);
    let trailing = SlotDisplaySummary {
        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,
        raw_payload: trailing,
        item_stack: None,
        tag: None,
    };
    assert!(trailing
        .decode_value()
        .unwrap_err()
        .to_string()
        .contains("trailing bytes after slot display value"));
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

    let mut trailing = Encoder::new();
    trailing.write_bool(false);
    trailing.write_u8(0);
    assert!(decode_play_clientbound(
        ids::play::CLIENTBOUND_SELECT_ADVANCEMENTS_TAB,
        &trailing.into_inner()
    )
    .unwrap_err()
    .to_string()
    .contains("trailing bytes after select advancements tab packet"));
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

    let mut bytes = payload.into_inner();
    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_UPDATE_ADVANCEMENTS, &bytes).unwrap(),
        PlayClientbound::UpdateAdvancements(UpdateAdvancements {
            reset: true,
            added: vec![AdvancementSummary {
                id: "minecraft:story/root".to_string(),
                parent: None,
                display: Some(AdvancementDisplaySummary {
                    title: RichTextComponent::plain("Root"),
                    description: RichTextComponent::plain("Description"),
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

    bytes.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_UPDATE_ADVANCEMENTS, &bytes)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after update advancements packet")
    );
}

#[test]
fn decodes_update_advancements_rich_display_components() {
    let title = styled_component_root("Root", "gold", true, " child");
    let description = styled_component_root("Description", "green", false, " detail");
    let mut payload = Encoder::new();
    payload.write_bool(false);
    payload.write_var_i32(1);
    payload.write_string("minecraft:story/root");
    payload.write_bool(false);
    payload.write_bool(true);
    payload.write_bytes(&title);
    payload.write_bytes(&description);
    payload.write_var_i32(42);
    payload.write_var_i32(1);
    payload.write_var_i32(0);
    payload.write_var_i32(0);
    payload.write_var_i32(1);
    payload.write_i32(2);
    payload.write_f32(0.0);
    payload.write_f32(0.0);
    payload.write_var_i32(1);
    payload.write_var_i32(1);
    payload.write_string("done");
    payload.write_bool(false);
    payload.write_var_i32(0);
    payload.write_var_i32(0);
    payload.write_bool(true);

    let PlayClientbound::UpdateAdvancements(packet) = decode_play_clientbound(
        ids::play::CLIENTBOUND_UPDATE_ADVANCEMENTS,
        &payload.into_inner(),
    )
    .unwrap() else {
        panic!("expected update advancements packet");
    };
    let display = packet.added[0].display.as_ref().unwrap();
    assert_eq!(display.title.text(), "Root child");
    assert_eq!(
        display.title.component(),
        &crate::decode_nbt_summary(&title).unwrap().unwrap()
    );
    assert_eq!(display.title.styled().len(), 1);
    assert_eq!(display.title.styled()[0].style.color, Some(0xFF_AA00));
    assert_eq!(display.title.styled()[0].style.bold, Some(true));
    assert_eq!(display.description.text(), "Description detail");
    assert_eq!(display.description.styled()[0].style.color, Some(0x55_FF55));
    assert_eq!(display.frame_type, AdvancementFrameType::Challenge);
    assert!(display.show_toast);
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
                    title: RichTextComponent::plain("Root"),
                    description: RichTextComponent::plain("Description"),
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
fn rejects_invalid_advancement_frame_type_ordinal() {
    let mut payload = Encoder::new();
    payload.write_bool(false);
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
    payload.write_var_i32(3);

    let error = decode_play_clientbound(
        ids::play::CLIENTBOUND_UPDATE_ADVANCEMENTS,
        &payload.into_inner(),
    )
    .unwrap_err();
    assert!(error
        .to_string()
        .contains("invalid advancement frame type ordinal 3"));
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

#[test]
fn rejects_compound_tag_query_with_trailing_bytes() {
    let mut payload = Encoder::new();
    payload.write_var_i32(14);
    payload.write_bytes(&compound_with_string("name", "Chest"));
    payload.write_u8(1);

    let err = decode_play_clientbound(ids::play::CLIENTBOUND_TAG_QUERY, &payload.into_inner())
        .unwrap_err();
    assert!(err
        .to_string()
        .contains("trailing bytes after tag query nbt"));
}

fn item_stack(item_id: i32, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: DataComponentPatchSummary::default(),
    }
}

fn item_slot_display_summary(item_id: i32) -> SlotDisplaySummary {
    SlotDisplaySummary {
        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,
        raw_payload: slot_display_item_payload(item_id),
        item_stack: Some(item_stack(item_id, 1)),
        tag: None,
    }
}

fn empty_slot_display_summary() -> SlotDisplaySummary {
    SlotDisplaySummary {
        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
        raw_payload: slot_display_type_payload(VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID),
        item_stack: None,
        tag: None,
    }
}

fn decode_place_ghost_recipe_display(raw_body: &[u8]) -> RecipeDisplaySummary {
    let mut payload = Encoder::new();
    payload.write_var_i32(9);
    payload.write_bytes(raw_body);
    let PlayClientbound::PlaceGhostRecipe(packet) = decode_play_clientbound(
        ids::play::CLIENTBOUND_PLACE_GHOST_RECIPE,
        &payload.into_inner(),
    )
    .unwrap() else {
        panic!("expected place ghost recipe packet");
    };
    packet.recipe_display
}

fn decode_update_recipes_option_display(option_display: Vec<u8>) -> SlotDisplaySummary {
    let PlayClientbound::UpdateRecipes(UpdateRecipes {
        mut stonecutter_recipes,
        ..
    }) = decode_play_clientbound(
        ids::play::CLIENTBOUND_UPDATE_RECIPES,
        &update_recipes_payload(&option_display),
    )
    .unwrap()
    else {
        panic!("expected update recipes packet");
    };
    stonecutter_recipes.remove(0).option_display
}

fn update_recipes_payload(option_display: &[u8]) -> Vec<u8> {
    let mut payload = Encoder::new();
    payload.write_var_i32(0);
    payload.write_var_i32(1);
    payload.write_var_i32(2);
    payload.write_var_i32(11);
    payload.write_bytes(option_display);
    payload.into_inner()
}

fn slot_display_type_payload(display_type_id: i32) -> Vec<u8> {
    let mut payload = Encoder::new();
    payload.write_var_i32(display_type_id);
    payload.into_inner()
}

fn slot_display_unary_payload(display_type_id: i32, display: &[u8]) -> Vec<u8> {
    let mut payload = Encoder::new();
    payload.write_var_i32(display_type_id);
    payload.write_bytes(display);
    payload.into_inner()
}

fn slot_display_binary_payload(display_type_id: i32, first: &[u8], second: &[u8]) -> Vec<u8> {
    let mut payload = Encoder::new();
    payload.write_var_i32(display_type_id);
    payload.write_bytes(first);
    payload.write_bytes(second);
    payload.into_inner()
}

fn slot_display_with_component_payload(display: &[u8], component_type_id: i32) -> Vec<u8> {
    let mut payload = Encoder::new();
    payload.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_ONLY_WITH_COMPONENT_ID);
    payload.write_bytes(display);
    payload.write_var_i32(component_type_id);
    payload.into_inner()
}

fn slot_display_smithing_trim_payload(
    base: &[u8],
    material: &[u8],
    encoded_pattern_holder_id: i32,
) -> Vec<u8> {
    let mut payload = Encoder::new();
    payload.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_SMITHING_TRIM_ID);
    payload.write_bytes(base);
    payload.write_bytes(material);
    payload.write_var_i32(encoded_pattern_holder_id);
    payload.into_inner()
}

fn slot_display_composite_payload(contents: &[Vec<u8>]) -> Vec<u8> {
    let mut payload = Encoder::new();
    payload.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_COMPOSITE_ID);
    payload.write_var_i32(contents.len() as i32);
    for content in contents {
        payload.write_bytes(content);
    }
    payload.into_inner()
}

fn slot_display_item_payload(item_id: i32) -> Vec<u8> {
    let mut payload = Encoder::new();
    payload.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID);
    payload.write_var_i32(item_id);
    payload.into_inner()
}

fn slot_display_item_stack_payload(item_id: i32, count: i32) -> Vec<u8> {
    let mut payload = Encoder::new();
    payload.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID);
    payload.write_var_i32(item_id);
    payload.write_var_i32(count);
    payload.write_var_i32(0);
    payload.write_var_i32(0);
    payload.into_inner()
}

fn slot_display_tag_payload(tag: &str) -> Vec<u8> {
    let mut payload = Encoder::new();
    payload.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_TAG_ID);
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

fn styled_component_root(text: &str, color: &str, bold: bool, extra: &str) -> Vec<u8> {
    let mut payload = vec![10, 8];
    write_mutf8(&mut payload, "text");
    write_mutf8(&mut payload, text);
    payload.push(8);
    write_mutf8(&mut payload, "color");
    write_mutf8(&mut payload, color);
    payload.push(1);
    write_mutf8(&mut payload, "bold");
    payload.push(u8::from(bold));
    payload.push(9);
    write_mutf8(&mut payload, "extra");
    payload.push(8);
    payload.extend_from_slice(&1i32.to_be_bytes());
    write_mutf8(&mut payload, extra);
    payload.push(0);
    payload
}

fn write_mutf8(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u16).to_be_bytes());
    out.extend_from_slice(value.as_bytes());
}
