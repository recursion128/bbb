use super::*;
use crate::{
    codec::{Decoder, Encoder},
    ids,
    packets::{decode_play_clientbound, DataComponentPatchSummary, PlayClientbound},
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
    payload.write_bytes(&[0xaa, 0xbb, 0xcc]);
    let payload = payload.into_inner();

    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_PLACE_GHOST_RECIPE, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::PlaceGhostRecipe(PlaceGhostRecipe {
            container_id: 9,
            recipe_display_type: RecipeDisplayType::Stonecutter,
            recipe_display_body: vec![0xaa, 0xbb, 0xcc],
        })
    );

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 9);
    assert_eq!(decoder.read_var_i32().unwrap(), 3);
    assert_eq!(
        decoder.read_exact(3, "recipe display body").unwrap(),
        &[0xaa, 0xbb, 0xcc]
    );
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
    payload.write_string("minecraft:furnace_input");
    payload.write_var_i32(2);
    payload.write_var_i32(42);
    payload.write_var_i32(43);
    payload.write_string("minecraft:smithing_base");
    payload.write_var_i32(1);
    payload.write_var_i32(99);

    payload.write_var_i32(1);
    payload.write_var_i32(3);
    payload.write_var_i32(11);
    payload.write_var_i32(12);
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
                    tag: None,
                    item_ids: vec![11, 12],
                },
                option_display: SlotDisplaySummary {
                    display_type_id: 4,
                    raw_payload: vec![4, 77],
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
    option_display.write_string("minecraft:test_trim");
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
                },
            }],
        })
    );
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
