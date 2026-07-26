use std::collections::{BTreeMap, BTreeSet};

use bbb_protocol::{
    builtin_enum_component_values::VANILLA_DYE_COLOR_RED_ID,
    codec::Encoder,
    data_component_types::{
        VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID, VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID,
        VANILLA_DATA_COMPONENT_DYE_TYPE_ID, VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
        VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID, VANILLA_DATA_COMPONENT_TRIM_TYPE_ID,
    },
    packets::{
        DataComponentPatchSummary, IngredientSummary, ItemStackSummary, PlaceGhostRecipe,
        RecipeDisplaySummary, RecipePropertySetSummary, RegistryTags, SlotDisplaySummary,
        StonecutterRecipeDisplaySummary, StonecutterSelectableRecipeSummary, TagNetworkPayload,
        TrimMaterialSummary, TrimPatternSummary, UpdateRecipes, UpdateTags,
    },
    potion_types::VANILLA_POTIONS,
    recipe_wire_types::{
        RecipeDisplayType, VANILLA_SLOT_DISPLAY_TYPE_ANY_FUEL_ID,
        VANILLA_SLOT_DISPLAY_TYPE_COMPOSITE_ID, VANILLA_SLOT_DISPLAY_TYPE_DYED_ID,
        VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID, VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,
        VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID, VANILLA_SLOT_DISPLAY_TYPE_ONLY_WITH_COMPONENT_ID,
        VANILLA_SLOT_DISPLAY_TYPE_SMITHING_TRIM_ID, VANILLA_SLOT_DISPLAY_TYPE_TAG_ID,
        VANILLA_SLOT_DISPLAY_TYPE_WITH_ANY_POTION_ID, VANILLA_SLOT_DISPLAY_TYPE_WITH_REMAINDER_ID,
    },
};

use crate::{RegistryPacketEntry, WorldStore, TRIM_MATERIAL_REGISTRY, TRIM_PATTERN_REGISTRY};

use super::*;

#[test]
fn basic_displays_follow_current_tag_fuel_and_source_order() {
    let mut world = WorldStore::new();
    world.set_furnace_fuel_item_ids(BTreeSet::from([9, 4]));
    set_item_tag(&mut world, "example:choices", vec![8, 3]);
    let display = SlotDisplayValue::Composite {
        contents: vec![
            SlotDisplayValue::Empty,
            item_display(6),
            SlotDisplayValue::Tag {
                tag: "example:choices".to_string(),
            },
            SlotDisplayValue::WithRemainder {
                input: Box::new(SlotDisplayValue::AnyFuel),
                remainder: Box::new(item_display(99)),
            },
        ],
    };

    assert_eq!(item_ids(resolve(&world, &display)), vec![6, 8, 3, 4, 9]);

    world.set_furnace_fuel_item_ids(BTreeSet::from([7]));
    set_item_tag(&mut world, "example:choices", vec![12]);
    assert_eq!(item_ids(resolve(&world, &display)), vec![6, 12, 7]);
    assert!(resolve(
        &world,
        &SlotDisplayValue::Tag {
            tag: "example:missing".to_string(),
        }
    )
    .is_empty());
}

#[test]
fn with_any_potion_uses_potion_outer_order_and_replaces_custom_contents() {
    let world = WorldStore::new();
    let source = SlotDisplayValue::Composite {
        contents: vec![
            item_stack_display(stack_with_patch(
                20,
                3,
                DataComponentPatchSummary {
                    added: 1,
                    added_type_ids: vec![VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID],
                    potion_id: Some(45),
                    potion_custom_color: Some(0x12_34_56),
                    potion_custom_effect_count: Some(2),
                    potion_custom_name: Some("old".to_string()),
                    ..DataComponentPatchSummary::default()
                },
            )),
            item_display(21),
        ],
    };

    let resolved = resolve(
        &world,
        &SlotDisplayValue::WithAnyPotion {
            display: Box::new(source),
        },
    );

    assert_eq!(resolved.len(), VANILLA_POTIONS.len() * 2);
    assert_eq!(item_ids(resolved[..4].to_vec()), vec![20, 21, 20, 21]);
    assert_eq!(resolved[0].count, 3);
    assert_eq!(resolved[0].component_patch.potion_id, Some(0));
    assert_eq!(resolved[1].component_patch.potion_id, Some(0));
    assert_eq!(resolved[2].component_patch.potion_id, Some(1));
    assert_eq!(resolved.last().unwrap().component_patch.potion_id, Some(45));
    assert_eq!(resolved[0].component_patch.potion_custom_color, None);
    assert_eq!(
        resolved[0].component_patch.potion_custom_effect_count,
        Some(0)
    );
    assert_eq!(resolved[0].component_patch.potion_custom_name, None);
}

#[test]
fn component_filter_honors_prototype_add_override_and_remove_precedence() {
    let mut world = WorldStore::new();
    world.set_default_item_component_type_ids(BTreeMap::from([
        (
            30,
            BTreeSet::from([VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID]),
        ),
        (
            32,
            BTreeSet::from([VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID]),
        ),
    ]));
    let explicit_add = DataComponentPatchSummary {
        added: 1,
        added_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID],
        ..DataComponentPatchSummary::default()
    };
    let explicit_remove = DataComponentPatchSummary {
        removed_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID],
        ..DataComponentPatchSummary::default()
    };
    let display = SlotDisplayValue::OnlyWithComponent {
        source: Box::new(SlotDisplayValue::Composite {
            contents: vec![
                item_display(30),
                item_stack_display(stack_with_patch(31, 1, explicit_add)),
                item_stack_display(stack_with_patch(32, 1, explicit_remove)),
                item_display(33),
            ],
        }),
        component_type_id: VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
    };

    assert_eq!(item_ids(resolve(&world, &display)), vec![30, 31]);
}

#[test]
fn dyed_display_uses_generated_diffuse_colors_dye_outer_order_and_limit() {
    let mut world = WorldStore::new();
    world.set_default_item_component_type_ids(BTreeMap::from([(
        40,
        BTreeSet::from([VANILLA_DATA_COMPONENT_DYE_TYPE_ID]),
    )]));
    world.set_default_item_dye_color_ids(BTreeMap::from([(40, VANILLA_DYE_COLOR_RED_ID)]));
    let current_color = DataComponentPatchSummary {
        added: 1,
        added_type_ids: vec![VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID],
        dyed_color: Some(0x33_66_99),
        ..DataComponentPatchSummary::default()
    };
    let targets = (0..9)
        .map(|index| {
            if index == 1 {
                item_stack_display(stack_with_patch(51, 4, current_color.clone()))
            } else {
                item_display(50 + index)
            }
        })
        .collect();
    let display = SlotDisplayValue::Dyed {
        dye: Box::new(SlotDisplayValue::Composite {
            contents: vec![item_display(40), item_display(41)],
        }),
        target: Box::new(SlotDisplayValue::Composite { contents: targets }),
    };

    let resolved = resolve(&world, &display);

    assert_eq!(resolved.len(), 16);
    assert_eq!(
        item_ids(resolved.clone()),
        vec![50, 51, 52, 53, 54, 55, 56, 57, 58, 50, 51, 52, 53, 54, 55, 56]
    );
    assert_eq!(resolved[0].component_patch.dyed_color, Some(0xB0_2E_26));
    assert_eq!(resolved[1].component_patch.dyed_color, Some(0xA4_6B_8A));
    assert_eq!(resolved[9].component_patch.dyed_color, Some(0xF9_FF_FE));
    assert_eq!(resolved[10].component_patch.dyed_color, Some(0x96_B2_CC));
    assert!(resolved.iter().all(|stack| stack.count == 1));
}

#[test]
fn smithing_trim_uses_reordered_dynamic_holders_and_stable_sampling() {
    let mut world = WorldStore::new();
    install_trim_registries(&mut world);
    let material_patch = DataComponentPatchSummary {
        added: 1,
        added_type_ids: vec![VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID],
        provides_trim_material_id: Some(1),
        ..DataComponentPatchSummary::default()
    };
    let display = SlotDisplayValue::SmithingTrim {
        base: Box::new(SlotDisplayValue::Composite {
            contents: vec![item_display(60), item_display(61)],
        }),
        material: Box::new(item_stack_display(stack_with_patch(62, 8, material_patch))),
        pattern: SlotDisplayTrimPatternSummary {
            registry_id: Some(0),
            direct: None,
        },
    };

    let first = SlotDisplayResolver { world: &world }
        .resolve(&display, 0x1234_5678)
        .unwrap();
    let second = SlotDisplayResolver { world: &world }
        .resolve(&display, 0x1234_5678)
        .unwrap();

    assert_eq!(first, second);
    assert_eq!(first.len(), 16);
    assert!(first.iter().all(|stack| {
        stack.count == 1
            && stack
                .component_patch
                .added_type_ids
                .contains(&VANILLA_DATA_COMPONENT_TRIM_TYPE_ID)
            && stack.component_patch.armor_trim_material_id == Some(1)
            && stack.component_patch.armor_trim_pattern_id == Some(0)
    }));

    let same_trim = DataComponentPatchSummary {
        added: 1,
        added_type_ids: vec![VANILLA_DATA_COMPONENT_TRIM_TYPE_ID],
        armor_trim_material_id: Some(1),
        armor_trim_pattern_id: Some(0),
        ..DataComponentPatchSummary::default()
    };
    let unchanged = SlotDisplayValue::SmithingTrim {
        base: Box::new(item_stack_display(stack_with_patch(60, 1, same_trim))),
        material: match display {
            SlotDisplayValue::SmithingTrim { material, .. } => material,
            _ => unreachable!(),
        },
        pattern: SlotDisplayTrimPatternSummary {
            registry_id: Some(0),
            direct: None,
        },
    };
    assert!(resolve(&world, &unchanged).is_empty());
}

#[test]
fn direct_trim_values_and_candidate_budget_are_validated() {
    let world = WorldStore::new();
    let material = TrimMaterialSummary {
        asset_name: "opal".to_string(),
        override_armor_assets: BTreeMap::new(),
        description: "Opal".to_string(),
        description_styled: None,
        description_nbt: None,
    };
    let pattern = TrimPatternSummary {
        asset_id: "example:spiral".to_string(),
        description: "Spiral".to_string(),
        description_styled: None,
        description_nbt: None,
        decal: true,
    };
    let material_patch = DataComponentPatchSummary {
        added: 1,
        added_type_ids: vec![VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID],
        provides_trim_material_direct: Some(Box::new(material.clone())),
        ..DataComponentPatchSummary::default()
    };
    let display = SlotDisplayValue::SmithingTrim {
        base: Box::new(item_display(70)),
        material: Box::new(item_stack_display(stack_with_patch(71, 1, material_patch))),
        pattern: SlotDisplayTrimPatternSummary {
            registry_id: None,
            direct: Some(pattern.clone()),
        },
    };
    let resolved = resolve(&world, &display);
    assert_eq!(resolved.len(), 16);
    assert_eq!(
        resolved[0]
            .component_patch
            .armor_trim_material_direct
            .as_deref(),
        Some(&material)
    );
    assert_eq!(
        resolved[0]
            .component_patch
            .armor_trim_pattern_direct
            .as_deref(),
        Some(&pattern)
    );

    let oversized_source = SlotDisplayValue::Composite {
        contents: (0..1425).map(|_| item_display(72)).collect(),
    };
    let error = SlotDisplayResolver { world: &world }
        .resolve(
            &SlotDisplayValue::WithAnyPotion {
                display: Box::new(oversized_source.clone()),
            },
            0,
        )
        .unwrap_err();
    assert_eq!(
        error,
        SlotDisplayResolveError::TooManyCandidates { actual: 65_550 }
    );
    let first = SlotDisplayResolver { world: &world }
        .resolve_first(
            &SlotDisplayValue::Composite {
                contents: vec![item_display(74), oversized_source],
            },
            0,
        )
        .unwrap()
        .unwrap();
    assert_eq!(first.item_id, Some(74));

    let invalid_material = SlotDisplayValue::ItemStack {
        stack: Box::new(stack_with_patch(
            73,
            1,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID],
                provides_trim_material_id: Some(99),
                ..DataComponentPatchSummary::default()
            },
        )),
    };
    assert!(!validate_slot_display_holders(&world, &invalid_material));
}

#[test]
fn packet_application_rejects_unresolved_trim_holders_atomically_but_allows_missing_display_tags() {
    let mut world = WorldStore::new();
    install_trim_registries(&mut world);
    world.apply_update_recipes(update_recipes_packet(
        "minecraft:first",
        smithing_summary(1),
    ));
    let accepted = world.recipes().clone();

    world.apply_update_recipes(update_recipes_packet(
        "minecraft:invalid",
        smithing_summary(3),
    ));
    assert_eq!(world.recipes(), &accepted);

    world.apply_update_recipes(update_recipes_packet(
        "minecraft:missing_tag",
        tag_summary("example:missing"),
    ));
    assert!(world
        .recipes()
        .property_sets
        .contains_key("minecraft:missing_tag"));
    assert!(world
        .resolve_slot_display_stacks(&world.recipes().stonecutter_recipes[0].option_display)
        .unwrap()
        .is_empty());
    assert_eq!(world.counters().update_recipes_packets, 3);
}

#[test]
fn place_ghost_recipe_snapshots_every_builtin_slot_display_semantic() {
    let mut world = WorldStore::new();
    assert!(world.open_local_inventory());
    world.set_furnace_fuel_item_ids(BTreeSet::from([9, 4]));
    set_item_tag(&mut world, "example:choices", vec![8, 3]);
    world.set_default_item_component_type_ids(BTreeMap::from([
        (
            30,
            BTreeSet::from([VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID]),
        ),
        (40, BTreeSet::from([VANILLA_DATA_COMPONENT_DYE_TYPE_ID])),
    ]));
    world.set_default_item_dye_color_ids(BTreeMap::from([(40, VANILLA_DYE_COLOR_RED_ID)]));
    install_trim_registries(&mut world);

    let item = encoded_item_display(55);
    let item_stack = encoded_item_stack_display(56, 2);
    let tag = encoded_tag_display("example:choices");
    let any_fuel = encoded_unit_display(VANILLA_SLOT_DISPLAY_TYPE_ANY_FUEL_ID);
    let with_any_potion =
        encoded_unary_display(VANILLA_SLOT_DISPLAY_TYPE_WITH_ANY_POTION_ID, &item);
    let only_with_component = encoded_with_component_display(
        &encoded_item_display(30),
        VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
    );
    let dyed = encoded_binary_display(
        VANILLA_SLOT_DISPLAY_TYPE_DYED_ID,
        &encoded_item_display(40),
        &encoded_item_display(50),
    );
    let smithing_trim = encoded_smithing_trim_display(
        &encoded_item_display(60),
        &encoded_trim_material_item_stack_display(62, 1),
        1,
    );
    let with_remainder = encoded_binary_display(
        VANILLA_SLOT_DISPLAY_TYPE_WITH_REMAINDER_ID,
        &encoded_item_display(70),
        &encoded_item_display(71),
    );
    let composite =
        encoded_composite_display(&[encoded_item_display(80), encoded_item_display(81)]);
    let cases = [
        (
            encoded_unit_display(VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID),
            Vec::new(),
        ),
        (any_fuel.clone(), vec![4, 9]),
        (
            with_any_potion.clone(),
            vec![55; bbb_protocol::potion_types::VANILLA_POTIONS.len()],
        ),
        (only_with_component, vec![30]),
        (item, vec![55]),
        (item_stack, vec![56]),
        (tag.clone(), vec![8, 3]),
        (dyed.clone(), vec![50]),
        (smithing_trim.clone(), vec![60; 16]),
        (with_remainder, vec![70]),
        (composite, vec![80, 81]),
    ];

    for (result, expected_item_ids) in cases {
        world.apply_place_ghost_recipe(PlaceGhostRecipe {
            container_id: 0,
            recipe_display: stonecutter_result_display(result),
        });
        let slots = &world.last_ghost_recipe().unwrap().slots;
        if expected_item_ids.is_empty() {
            assert!(slots.is_empty());
            continue;
        }
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].slot_id, 0);
        assert!(slots[0].is_result);
        assert_eq!(item_ids(slots[0].items.clone()), expected_item_ids);
    }

    world.apply_place_ghost_recipe(PlaceGhostRecipe {
        container_id: 0,
        recipe_display: stonecutter_result_display(with_any_potion),
    });
    let potion_items = &world.last_ghost_recipe().unwrap().slots[0].items;
    assert_eq!(potion_items[0].component_patch.potion_id, Some(0));
    assert_eq!(
        potion_items.last().unwrap().component_patch.potion_id,
        Some(45)
    );

    world.apply_place_ghost_recipe(PlaceGhostRecipe {
        container_id: 0,
        recipe_display: stonecutter_result_display(dyed),
    });
    let dyed_item = &world.last_ghost_recipe().unwrap().slots[0].items[0];
    assert_eq!(dyed_item.component_patch.dyed_color, Some(0xB0_2E_26));

    world.apply_place_ghost_recipe(PlaceGhostRecipe {
        container_id: 0,
        recipe_display: stonecutter_result_display(smithing_trim),
    });
    let trimmed_items = &world.last_ghost_recipe().unwrap().slots[0].items;
    assert!(trimmed_items.iter().all(|stack| {
        stack.component_patch.armor_trim_material_id == Some(1)
            && stack.component_patch.armor_trim_pattern_id == Some(0)
    }));
}

#[test]
fn place_ghost_recipe_keeps_candidate_snapshots_and_rejects_invalid_updates_atomically() {
    let mut world = WorldStore::new();
    assert!(world.open_local_inventory());
    install_trim_registries(&mut world);
    set_item_tag(&mut world, "example:choices", vec![8, 3]);

    world.apply_place_ghost_recipe(PlaceGhostRecipe {
        container_id: 0,
        recipe_display: stonecutter_result_display(encoded_tag_display("example:choices")),
    });
    let tag_snapshot = world.last_ghost_recipe().unwrap().clone();
    set_item_tag(&mut world, "example:choices", vec![12]);
    assert_eq!(world.last_ghost_recipe(), Some(&tag_snapshot));

    world.set_furnace_fuel_item_ids(BTreeSet::from([9, 4]));
    world.apply_place_ghost_recipe(PlaceGhostRecipe {
        container_id: 0,
        recipe_display: stonecutter_result_display(encoded_unit_display(
            VANILLA_SLOT_DISPLAY_TYPE_ANY_FUEL_ID,
        )),
    });
    let fuel_snapshot = world.last_ghost_recipe().unwrap().clone();
    world.set_furnace_fuel_item_ids(BTreeSet::from([7]));
    assert_eq!(world.last_ghost_recipe(), Some(&fuel_snapshot));

    let ignored_before = world.counters().ghost_recipe_updates_ignored;
    world.apply_place_ghost_recipe(PlaceGhostRecipe {
        container_id: 0,
        recipe_display: stonecutter_result_display(encoded_smithing_trim_display(
            &encoded_item_display(60),
            &encoded_trim_material_item_stack_display(62, 1),
            3,
        )),
    });
    assert_eq!(world.last_ghost_recipe(), Some(&fuel_snapshot));
    assert_eq!(
        world.counters().ghost_recipe_updates_ignored,
        ignored_before + 1
    );

    let oversized_source =
        encoded_composite_display(&(0..1425).map(encoded_item_display).collect::<Vec<_>>());
    world.apply_place_ghost_recipe(PlaceGhostRecipe {
        container_id: 0,
        recipe_display: stonecutter_result_display(encoded_unary_display(
            VANILLA_SLOT_DISPLAY_TYPE_WITH_ANY_POTION_ID,
            &oversized_source,
        )),
    });
    assert_eq!(world.last_ghost_recipe(), Some(&fuel_snapshot));
    assert_eq!(
        world.counters().ghost_recipe_updates_ignored,
        ignored_before + 2
    );
}

#[test]
fn default_component_tables_round_trip_and_publications_remain_isolated() {
    let mut world = WorldStore::new();
    let generation_before = world.publication_snapshot();
    world.set_default_item_component_type_ids(BTreeMap::from([(
        80,
        BTreeSet::from([
            VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
            VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
        ]),
    )]));
    world.set_default_item_dye_color_ids(BTreeMap::from([(80, VANILLA_DYE_COLOR_RED_ID)]));
    world.set_default_item_potion_duration_scale_bits(BTreeMap::from([(80, 0.125_f32.to_bits())]));
    let generation_after = world.publication_snapshot();
    let dye = item_display(80);

    assert!(!generation_before
        .item_stack_has_effective_component(item_stack(&dye), VANILLA_DATA_COMPONENT_DYE_TYPE_ID));
    assert_eq!(
        resolve(&generation_after, &dye)[0]
            .component_patch
            .dye_color_id,
        Some(VANILLA_DYE_COLOR_RED_ID)
    );

    world.reset_registry_session();
    assert_eq!(
        resolve(&world, &dye)[0].component_patch.dye_color_id,
        Some(VANILLA_DYE_COLOR_RED_ID)
    );
    let serialized = serde_json::to_vec(&world).unwrap();
    let json: serde_json::Value = serde_json::from_slice(&serialized).unwrap();
    assert!(json["default_component_type_ids"].is_object());
    assert!(json["default_dye_color_ids"].is_object());
    assert!(json["default_potion_duration_scale_bits"].is_object());
    let restored: WorldStore = serde_json::from_value(json).unwrap();
    assert_eq!(
        resolve(&restored, &dye)[0].component_patch.dye_color_id,
        Some(VANILLA_DYE_COLOR_RED_ID)
    );
    assert_eq!(
        resolve(&restored, &dye)[0]
            .component_patch
            .potion_duration_scale_bits,
        Some(0.125_f32.to_bits())
    );

    let mut legacy_json = serde_json::to_value(&world).unwrap();
    let legacy_object = legacy_json.as_object_mut().unwrap();
    legacy_object.remove("default_component_type_ids");
    legacy_object.remove("default_dye_color_ids");
    legacy_object.remove("default_potion_duration_scale_bits");
    let restored_legacy: WorldStore = serde_json::from_value(legacy_json).unwrap();
    assert!(!restored_legacy
        .item_stack_has_effective_component(item_stack(&dye), VANILLA_DATA_COMPONENT_DYE_TYPE_ID));
}

fn resolve(world: &WorldStore, display: &SlotDisplayValue) -> Vec<ItemStackSummary> {
    SlotDisplayResolver { world }.resolve(display, 7).unwrap()
}

fn item_ids(stacks: Vec<ItemStackSummary>) -> Vec<i32> {
    stacks
        .into_iter()
        .map(|stack| stack.item_id.unwrap())
        .collect()
}

fn item_display(item_id: i32) -> SlotDisplayValue {
    SlotDisplayValue::Item {
        stack: Box::new(stack_with_patch(
            item_id,
            1,
            DataComponentPatchSummary::default(),
        )),
    }
}

fn item_stack_display(stack: ItemStackSummary) -> SlotDisplayValue {
    SlotDisplayValue::ItemStack {
        stack: Box::new(stack),
    }
}

fn item_stack(display: &SlotDisplayValue) -> &ItemStackSummary {
    match display {
        SlotDisplayValue::Item { stack } | SlotDisplayValue::ItemStack { stack } => stack,
        _ => panic!("expected item display"),
    }
}

fn stack_with_patch(
    item_id: i32,
    count: i32,
    component_patch: DataComponentPatchSummary,
) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch,
    }
}

fn set_item_tag(world: &mut WorldStore, tag: &str, entries: Vec<i32>) {
    world.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![TagNetworkPayload {
                tag: tag.to_string(),
                entries,
            }],
        }],
    });
}

fn install_trim_registries(world: &mut WorldStore) {
    world.record_registry_entries(
        TRIM_MATERIAL_REGISTRY,
        0,
        vec![
            RegistryPacketEntry::with_raw_data("example:copper", material_nbt("copper", "Copper")),
            RegistryPacketEntry::with_raw_data(
                "minecraft:diamond",
                material_nbt("diamond", "Diamond"),
            ),
        ],
    );
    world.record_registry_entries(
        TRIM_PATTERN_REGISTRY,
        0,
        vec![RegistryPacketEntry::with_raw_data(
            "example:spiral",
            pattern_nbt("example:spiral", "Spiral", true),
        )],
    );
}

fn material_nbt(asset_name: &str, description: &str) -> Vec<u8> {
    let mut out = vec![10];
    write_nbt_string_field(&mut out, "asset_name", asset_name);
    write_description(&mut out, description);
    out.push(0);
    out
}

fn pattern_nbt(asset_id: &str, description: &str, decal: bool) -> Vec<u8> {
    let mut out = vec![10];
    write_nbt_string_field(&mut out, "asset_id", asset_id);
    write_description(&mut out, description);
    out.push(1);
    write_nbt_string(&mut out, "decal");
    out.push(u8::from(decal));
    out.push(0);
    out
}

fn write_description(out: &mut Vec<u8>, text: &str) {
    out.push(10);
    write_nbt_string(out, "description");
    write_nbt_string_field(out, "text", text);
    out.push(0);
}

fn write_nbt_string_field(out: &mut Vec<u8>, name: &str, value: &str) {
    out.push(8);
    write_nbt_string(out, name);
    write_nbt_string(out, value);
}

fn write_nbt_string(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u16).to_be_bytes());
    out.extend_from_slice(value.as_bytes());
}

fn update_recipes_packet(property_key: &str, option_display: SlotDisplaySummary) -> UpdateRecipes {
    UpdateRecipes {
        property_sets: vec![RecipePropertySetSummary {
            key: property_key.to_string(),
            item_ids: vec![1],
        }],
        stonecutter_recipes: vec![StonecutterSelectableRecipeSummary {
            input: IngredientSummary {
                tag: None,
                item_ids: vec![1],
            },
            option_display,
        }],
    }
}

fn smithing_summary(encoded_pattern_holder: i32) -> SlotDisplaySummary {
    let mut payload = Encoder::new();
    payload.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_SMITHING_TRIM_ID);
    payload.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID);
    payload.write_var_i32(2);
    payload.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID);
    payload.write_var_i32(3);
    payload.write_var_i32(encoded_pattern_holder);
    SlotDisplaySummary {
        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_SMITHING_TRIM_ID,
        raw_payload: payload.into_inner(),
        item_stack: None,
        tag: None,
    }
}

fn tag_summary(tag: &str) -> SlotDisplaySummary {
    let mut payload = Encoder::new();
    payload.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_TAG_ID);
    payload.write_string(tag);
    SlotDisplaySummary {
        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_TAG_ID,
        raw_payload: payload.into_inner(),
        item_stack: None,
        tag: Some(tag.to_string()),
    }
}

fn stonecutter_result_display(result: SlotDisplaySummary) -> RecipeDisplaySummary {
    RecipeDisplaySummary {
        display_type: RecipeDisplayType::Stonecutter,
        raw_body: Vec::new(),
        crafting: None,
        furnace: None,
        stonecutter: Some(StonecutterRecipeDisplaySummary {
            input: encoded_unit_display(VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID),
            result,
            crafting_station: encoded_unit_display(VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID),
        }),
        smithing: None,
    }
}

fn encoded_unit_display(display_type_id: i32) -> SlotDisplaySummary {
    let mut payload = Encoder::new();
    payload.write_var_i32(display_type_id);
    encoded_slot_display(display_type_id, payload.into_inner())
}

fn encoded_unary_display(display_type_id: i32, display: &SlotDisplaySummary) -> SlotDisplaySummary {
    let mut payload = Encoder::new();
    payload.write_var_i32(display_type_id);
    payload.write_bytes(&display.raw_payload);
    encoded_slot_display(display_type_id, payload.into_inner())
}

fn encoded_binary_display(
    display_type_id: i32,
    first: &SlotDisplaySummary,
    second: &SlotDisplaySummary,
) -> SlotDisplaySummary {
    let mut payload = Encoder::new();
    payload.write_var_i32(display_type_id);
    payload.write_bytes(&first.raw_payload);
    payload.write_bytes(&second.raw_payload);
    encoded_slot_display(display_type_id, payload.into_inner())
}

fn encoded_with_component_display(
    source: &SlotDisplaySummary,
    component_type_id: i32,
) -> SlotDisplaySummary {
    let mut payload = Encoder::new();
    payload.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_ONLY_WITH_COMPONENT_ID);
    payload.write_bytes(&source.raw_payload);
    payload.write_var_i32(component_type_id);
    encoded_slot_display(
        VANILLA_SLOT_DISPLAY_TYPE_ONLY_WITH_COMPONENT_ID,
        payload.into_inner(),
    )
}

fn encoded_smithing_trim_display(
    base: &SlotDisplaySummary,
    material: &SlotDisplaySummary,
    encoded_pattern_holder: i32,
) -> SlotDisplaySummary {
    let mut payload = Encoder::new();
    payload.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_SMITHING_TRIM_ID);
    payload.write_bytes(&base.raw_payload);
    payload.write_bytes(&material.raw_payload);
    payload.write_var_i32(encoded_pattern_holder);
    encoded_slot_display(
        VANILLA_SLOT_DISPLAY_TYPE_SMITHING_TRIM_ID,
        payload.into_inner(),
    )
}

fn encoded_composite_display(contents: &[SlotDisplaySummary]) -> SlotDisplaySummary {
    let mut payload = Encoder::new();
    payload.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_COMPOSITE_ID);
    payload.write_var_i32(contents.len() as i32);
    for content in contents {
        payload.write_bytes(&content.raw_payload);
    }
    encoded_slot_display(VANILLA_SLOT_DISPLAY_TYPE_COMPOSITE_ID, payload.into_inner())
}

fn encoded_item_display(item_id: i32) -> SlotDisplaySummary {
    let mut payload = Encoder::new();
    payload.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID);
    payload.write_var_i32(item_id);
    encoded_slot_display(VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID, payload.into_inner())
}

fn encoded_item_stack_display(item_id: i32, count: i32) -> SlotDisplaySummary {
    let mut payload = Encoder::new();
    payload.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID);
    payload.write_var_i32(item_id);
    payload.write_var_i32(count);
    payload.write_var_i32(0);
    payload.write_var_i32(0);
    encoded_slot_display(
        VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID,
        payload.into_inner(),
    )
}

fn encoded_trim_material_item_stack_display(
    item_id: i32,
    material_registry_id: i32,
) -> SlotDisplaySummary {
    let mut payload = Encoder::new();
    payload.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID);
    payload.write_var_i32(item_id);
    payload.write_var_i32(1);
    payload.write_var_i32(1);
    payload.write_var_i32(0);
    payload.write_var_i32(VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID);
    payload.write_var_i32(material_registry_id + 1);
    encoded_slot_display(
        VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID,
        payload.into_inner(),
    )
}

fn encoded_tag_display(tag: &str) -> SlotDisplaySummary {
    let mut payload = Encoder::new();
    payload.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_TAG_ID);
    payload.write_string(tag);
    let mut display = encoded_slot_display(VANILLA_SLOT_DISPLAY_TYPE_TAG_ID, payload.into_inner());
    display.tag = Some(tag.to_string());
    display
}

fn encoded_slot_display(display_type_id: i32, raw_payload: Vec<u8>) -> SlotDisplaySummary {
    SlotDisplaySummary {
        display_type_id,
        raw_payload,
        item_stack: None,
        tag: None,
    }
}
