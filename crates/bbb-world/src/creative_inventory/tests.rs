use std::collections::BTreeMap;

use bbb_protocol::{
    data_component_types::{
        VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID, VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID,
        VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID, VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
        VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID,
        VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID,
    },
    item_component_value_types::ItemRaritySummary,
    item_types::{
        vanilla_item_type, vanilla_item_type_for_resource_id, VANILLA_ITEM_DIAMOND_ID,
        VANILLA_ITEM_STONE_ID, VANILLA_SUSPICIOUS_STEW_INGREDIENTS,
    },
    packets::{
        ContainerSetSlot, DataComponentPatchSummary, ItemStackSummary, PlayerAbilities,
        SetCursorItem, UpdateEnabledFeatures,
    },
};

use crate::{
    painting_variants::VANILLA_PAINTING_VARIANTS, LocalPlayerAbilitiesState,
    LocalPlayerPermissionLevel, RegistryContentState, RegistryPacketEntry, RegistryTagState,
    VanillaEnchantmentIdentity, WorldStore, INSTRUMENT_REGISTRY, PAINTING_VARIANT_REGISTRY,
};

use super::CreativeInventoryTabType;

const REGULAR_GOAT_HORNS: &str = "minecraft:regular_goat_horns";
const SCREAMING_GOAT_HORNS: &str = "minecraft:screaming_goat_horns";
const GOAT_HORNS: &str = "minecraft:goat_horns";

fn item_stack(item_id: i32, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: DataComponentPatchSummary::default(),
    }
}

fn item_id(resource_id: &str) -> i32 {
    vanilla_item_type_for_resource_id(resource_id).unwrap().id
}

fn creative_abilities() -> LocalPlayerAbilitiesState {
    LocalPlayerAbilitiesState {
        invulnerable: true,
        flying: false,
        can_fly: true,
        instabuild: true,
        flying_speed: 0.05,
        walking_speed: 0.1,
    }
}

fn world_with_instrument_registry() -> WorldStore {
    let mut world = WorldStore::new();
    let resources = [
        "minecraft:ponder_goat_horn",
        "minecraft:sing_goat_horn",
        "minecraft:seek_goat_horn",
        "minecraft:feel_goat_horn",
        "minecraft:admire_goat_horn",
        "minecraft:call_goat_horn",
        "minecraft:yearn_goat_horn",
        "minecraft:dream_goat_horn",
    ];
    world.registries.contents.insert(
        INSTRUMENT_REGISTRY.to_string(),
        RegistryContentState {
            registry: INSTRUMENT_REGISTRY.to_string(),
            packet_count: 1,
            entries: resources
                .into_iter()
                .map(|resource| RegistryPacketEntry {
                    id: resource.to_string(),
                    has_data: false,
                    raw_data_len: 0,
                    nbt: None,
                    raw_data: None,
                    known_pack_data: None,
                    element_data: None,
                    element_data_error: None,
                })
                .collect(),
            duplicate_entry_ids: BTreeMap::new(),
        },
    );
    world.registries.tags.insert(
        INSTRUMENT_REGISTRY.to_string(),
        RegistryTagState {
            registry: INSTRUMENT_REGISTRY.to_string(),
            tags: BTreeMap::from([
                (GOAT_HORNS.to_string(), (0..=7).collect()),
                (REGULAR_GOAT_HORNS.to_string(), vec![0, 1, 2, 3]),
                (SCREAMING_GOAT_HORNS.to_string(), vec![4, 5, 6, 7]),
            ]),
        },
    );
    world.registries.contents.insert(
        PAINTING_VARIANT_REGISTRY.to_string(),
        RegistryContentState {
            registry: PAINTING_VARIANT_REGISTRY.to_string(),
            packet_count: 1,
            entries: VANILLA_PAINTING_VARIANTS
                .iter()
                .map(|variant| RegistryPacketEntry {
                    id: variant.resource_id.to_string(),
                    has_data: false,
                    raw_data_len: 0,
                    nbt: None,
                    raw_data: None,
                    known_pack_data: None,
                    element_data: None,
                    element_data_error: None,
                })
                .collect(),
            duplicate_entry_ids: BTreeMap::new(),
        },
    );
    world.registries.tags.insert(
        PAINTING_VARIANT_REGISTRY.to_string(),
        RegistryTagState {
            registry: PAINTING_VARIANT_REGISTRY.to_string(),
            tags: BTreeMap::from([(
                "minecraft:placeable".to_string(),
                VANILLA_PAINTING_VARIANTS
                    .iter()
                    .filter(|variant| {
                        !matches!(
                            variant.resource_id,
                            "minecraft:earth"
                                | "minecraft:wind"
                                | "minecraft:water"
                                | "minecraft:fire"
                        )
                    })
                    .map(|variant| variant.id)
                    .collect(),
            )]),
        },
    );
    world
}

fn added_component_count(tab: &crate::CreativeInventoryTabSnapshot, type_id: i32) -> usize {
    tab.items
        .iter()
        .filter(|stack| stack.component_patch.added_type_ids.contains(&type_id))
        .count()
}

fn painting_ids(placeable: bool) -> Vec<i32> {
    let mut paintings: Vec<_> = VANILLA_PAINTING_VARIANTS
        .iter()
        .filter(|variant| {
            let is_placeable = !matches!(
                variant.resource_id,
                "minecraft:earth" | "minecraft:wind" | "minecraft:water" | "minecraft:fire"
            );
            is_placeable == placeable
        })
        .collect();
    paintings.sort_by_key(|variant| (variant.width * variant.height, variant.width));
    paintings.into_iter().map(|variant| variant.id).collect()
}

#[test]
fn materializes_all_dynamic_families_and_search_union_in_vanilla_order() {
    let mut world = world_with_instrument_registry();
    assert!(world.rebuild_creative_inventory(false).unwrap());
    let snapshot = world.creative_inventory_snapshot().unwrap();
    let accepted_generation = snapshot.generation;

    assert_eq!(snapshot.tabs.len(), 13);
    assert!(snapshot.tab("minecraft:op_blocks").is_none());
    assert_eq!(
        snapshot
            .tabs
            .iter()
            .map(|tab| tab.resource_id.as_str())
            .collect::<Vec<_>>(),
        [
            "minecraft:building_blocks",
            "minecraft:colored_blocks",
            "minecraft:natural_blocks",
            "minecraft:functional_blocks",
            "minecraft:redstone_blocks",
            "minecraft:hotbar",
            "minecraft:search",
            "minecraft:tools_and_utilities",
            "minecraft:combat",
            "minecraft:food_and_drinks",
            "minecraft:ingredients",
            "minecraft:spawn_eggs",
            "minecraft:inventory",
        ]
    );

    let tools = snapshot.tab("minecraft:tools_and_utilities").unwrap();
    assert_eq!(
        added_component_count(tools, VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID),
        2
    );
    assert_eq!(
        added_component_count(tools, VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID),
        7
    );
    assert_eq!(
        tools
            .items
            .iter()
            .filter(|stack| stack.item_id == Some(item_id("minecraft:firework_rocket")))
            .map(|stack| stack.component_patch.fireworks_flight_duration.unwrap_or(1))
            .collect::<Vec<_>>(),
        [1, 2, 3]
    );
    assert_eq!(
        tools
            .items
            .iter()
            .filter(|stack| stack.item_id == Some(item_id("minecraft:goat_horn")))
            .map(|stack| stack.component_patch.instrument_id.unwrap_or(0))
            .collect::<Vec<_>>(),
        (0..=7).collect::<Vec<_>>()
    );
    let food = snapshot.tab("minecraft:food_and_drinks").unwrap();
    assert_eq!(
        added_component_count(food, VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID),
        138
    );
    assert_eq!(
        added_component_count(
            food,
            VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID
        ),
        4
    );
    let potion_holders: Vec<_> = food
        .items
        .iter()
        .filter_map(|stack| stack.component_patch.potion_id)
        .collect();
    let expected_potion_holders: Vec<_> = (0..=45).collect();
    assert_eq!(potion_holders.len(), 138);
    assert!(potion_holders
        .chunks_exact(46)
        .all(|holders| holders == expected_potion_holders));
    assert_eq!(
        food.items
            .iter()
            .filter(|stack| stack.item_id == Some(item_id("minecraft:ominous_bottle")))
            .map(|stack| { stack.component_patch.ominous_bottle_amplifier.unwrap_or(0) })
            .collect::<Vec<_>>(),
        [0, 1, 2, 3, 4]
    );
    let mut expected_stew_effects = Vec::new();
    for ingredient in VANILLA_SUSPICIOUS_STEW_INGREDIENTS {
        let effect = (ingredient.effect_type_id, ingredient.duration_ticks);
        if !expected_stew_effects.contains(&effect) {
            expected_stew_effects.push(effect);
        }
    }
    assert_eq!(
        food.items
            .iter()
            .filter(|stack| {
                stack.component_patch.added_type_ids
                    == [VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID]
            })
            .map(|stack| {
                let effect = &stack.component_patch.suspicious_stew_effects[0];
                (effect.effect_id, effect.duration)
            })
            .collect::<Vec<_>>(),
        expected_stew_effects
    );
    let functional = snapshot.tab("minecraft:functional_blocks").unwrap();
    assert_eq!(
        added_component_count(functional, VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID),
        47
    );
    assert_eq!(
        functional
            .items
            .iter()
            .filter_map(|stack| stack.component_patch.painting_variant_id)
            .collect::<Vec<_>>(),
        painting_ids(true)
    );
    let ominous_banner = functional
        .items
        .iter()
        .find(|stack| {
            stack
                .component_patch
                .added_type_ids
                .contains(&VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID)
        })
        .unwrap();
    assert_eq!(
        ominous_banner.component_patch.banner_pattern_layers.len(),
        8
    );
    assert_eq!(
        ominous_banner
            .component_patch
            .banner_pattern_layers
            .iter()
            .map(|layer| layer.color_id)
            .collect::<Vec<_>>(),
        [9, 8, 7, 8, 15, 8, 8, 15]
    );
    assert_eq!(
        ominous_banner
            .component_patch
            .tooltip_hidden_component_type_ids,
        [VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID]
    );
    assert_eq!(
        ominous_banner.component_patch.rarity,
        Some(ItemRaritySummary::Uncommon)
    );
    let ingredients = snapshot.tab("minecraft:ingredients").unwrap();
    assert_eq!(
        added_component_count(
            ingredients,
            VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID
        ),
        43
    );
    assert_eq!(
        ingredients
            .items
            .iter()
            .filter_map(|stack| stack.component_patch.stored_enchantments.first())
            .map(|enchantment| (enchantment.holder_id, enchantment.level))
            .collect::<Vec<_>>(),
        (0..=42)
            .map(|holder_id| {
                (
                    holder_id,
                    VanillaEnchantmentIdentity::for_vanilla_registry_id(holder_id)
                        .unwrap()
                        .max_level(),
                )
            })
            .collect::<Vec<_>>()
    );

    let search = snapshot.tab("minecraft:search").unwrap();
    assert_eq!(search.tab_type, CreativeInventoryTabType::Search);
    for (index, stack) in search.items.iter().enumerate() {
        assert!(
            !search.items[..index].contains(stack),
            "search stack {index} was not type-and-components de-duplicated"
        );
    }
    assert_eq!(
        search
            .items
            .iter()
            .filter(|stack| stack.item_id == Some(VANILLA_ITEM_STONE_ID))
            .count(),
        1
    );
    let mut expected_search_enchantments = Vec::new();
    for holder_id in 0..=42 {
        let max_level = VanillaEnchantmentIdentity::for_vanilla_registry_id(holder_id)
            .unwrap()
            .max_level();
        for level in 1..=max_level {
            expected_search_enchantments.push((holder_id, level));
        }
    }
    assert_eq!(
        search
            .items
            .iter()
            .filter_map(|stack| stack.component_patch.stored_enchantments.first())
            .map(|enchantment| (enchantment.holder_id, enchantment.level))
            .collect::<Vec<_>>(),
        expected_search_enchantments
    );
    assert!(!world.rebuild_creative_inventory(false).unwrap());
    assert_eq!(
        world.creative_inventory_snapshot().unwrap().generation,
        accepted_generation
    );
}

#[test]
fn permission_and_feature_gates_control_category_publication() {
    let mut world = world_with_instrument_registry();
    world.local_player_id = Some(7);
    world.local_player.permission_level = LocalPlayerPermissionLevel::Gamemaster;
    world.local_player.abilities = Some(creative_abilities());
    assert!(world.rebuild_creative_inventory(true).unwrap());
    let operator = world
        .creative_inventory_snapshot()
        .unwrap()
        .tab("minecraft:op_blocks")
        .unwrap();
    assert_eq!(
        added_component_count(operator, VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID),
        18
    );
    assert_eq!(
        added_component_count(operator, VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID),
        4
    );
    assert_eq!(
        operator
            .items
            .iter()
            .filter_map(|stack| stack.component_patch.painting_variant_id)
            .collect::<Vec<_>>(),
        painting_ids(false)
    );
    let test_block_modes: Vec<_> = operator
        .items
        .iter()
        .filter(|stack| stack.item_id == Some(item_id("minecraft:test_block")))
        .map(|stack| {
            stack
                .component_patch
                .block_state_properties
                .get("mode")
                .cloned()
                .unwrap_or_else(|| "start".to_string())
        })
        .collect();
    assert_eq!(test_block_modes, ["start", "log", "fail", "accept"]);
    let light_levels: Vec<_> = operator
        .items
        .iter()
        .filter(|stack| stack.item_id == Some(item_id("minecraft:light")))
        .map(|stack| {
            stack
                .component_patch
                .block_state_properties
                .get("level")
                .cloned()
                .unwrap_or_else(|| "15".to_string())
        })
        .collect();
    assert_eq!(
        light_levels,
        (0..=15).rev().map(|v| v.to_string()).collect::<Vec<_>>()
    );

    world.apply_update_enabled_features(UpdateEnabledFeatures {
        features: Vec::new(),
    });
    assert!(world.rebuild_creative_inventory(true).unwrap());
    let snapshot = world.creative_inventory_snapshot().unwrap();
    assert_eq!(
        snapshot
            .tabs
            .iter()
            .map(|tab| tab.resource_id.as_str())
            .collect::<Vec<_>>(),
        [
            "minecraft:hotbar",
            "minecraft:search",
            "minecraft:inventory"
        ]
    );
    assert!(snapshot.tab("minecraft:search").unwrap().items.is_empty());
}

#[test]
fn open_screen_rebuilds_on_permission_and_feature_revision_edges() {
    let mut world = world_with_instrument_registry();
    world.local_player_id = Some(7);
    world.local_player.abilities = Some(creative_abilities());
    world.open_local_inventory_with_operator_tab(true).unwrap();
    let initial_generation = world
        .creative_inventory_screen()
        .unwrap()
        .catalog_generation;
    assert!(world
        .creative_inventory_snapshot()
        .unwrap()
        .tab("minecraft:op_blocks")
        .is_none());

    world.apply_local_player_permission_entity_event(7, 26);
    let permission_generation = world
        .creative_inventory_screen()
        .unwrap()
        .catalog_generation;
    assert!(permission_generation > initial_generation);
    assert!(world
        .creative_inventory_snapshot()
        .unwrap()
        .tab("minecraft:op_blocks")
        .is_some());

    world.apply_update_enabled_features(UpdateEnabledFeatures {
        features: Vec::new(),
    });
    let feature_generation = world
        .creative_inventory_screen()
        .unwrap()
        .catalog_generation;
    assert!(feature_generation > permission_generation);
    assert_eq!(world.creative_inventory_snapshot().unwrap().tabs.len(), 3);
}

#[test]
fn failed_rebuild_retains_last_accepted_catalog() {
    let mut world = world_with_instrument_registry();
    world.rebuild_creative_inventory(false).unwrap();
    let accepted = world.creative_inventory_snapshot().unwrap().clone();
    world
        .registries
        .tags
        .get_mut(INSTRUMENT_REGISTRY)
        .unwrap()
        .tags
        .get_mut(GOAT_HORNS)
        .unwrap()
        .push(99);

    let error = world.rebuild_creative_inventory(false).unwrap_err();
    assert!(error
        .to_string()
        .contains("unresolved instrument holder id 99"));
    assert_eq!(world.creative_inventory_snapshot(), Some(&accepted));
    let error_text = error.to_string();
    assert_eq!(
        world.creative_inventory_build_error(),
        Some(error_text.as_str())
    );
}

#[test]
fn creative_screen_opens_for_infinite_materials_and_reverts_on_ability_loss() {
    let mut world = world_with_instrument_registry();
    world.local_player.abilities = Some(creative_abilities());

    assert!(world.open_local_inventory_with_operator_tab(false).unwrap());
    assert!(world.local_inventory_is_open());
    assert!(world.creative_inventory_screen().is_some());

    world.apply_player_abilities(PlayerAbilities {
        invulnerable: false,
        flying: false,
        can_fly: false,
        instabuild: false,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
    assert!(world.local_inventory_is_open());
    assert!(world.creative_inventory_screen().is_none());
}

#[test]
fn generic_inventory_open_falls_back_without_publishing_a_failed_creative_catalog() {
    let mut world = WorldStore::new();
    world.local_player.abilities = Some(creative_abilities());

    assert!(world.open_local_inventory());

    assert!(world.local_inventory_is_open());
    assert!(world.creative_inventory_screen().is_none());
    assert!(world.creative_inventory_snapshot().is_none());
    assert!(world.creative_inventory_build_error().is_some());
}

#[test]
fn creative_screen_suppresses_cursor_updates_and_resets_with_screen_replacement() {
    let mut world = world_with_instrument_registry();
    world.local_player.abilities = Some(creative_abilities());
    world.open_local_inventory_with_operator_tab(false).unwrap();
    world.inventory.cursor_item = item_stack(VANILLA_ITEM_STONE_ID, 1);

    world.apply_set_cursor_item(SetCursorItem {
        item: item_stack(VANILLA_ITEM_DIAMOND_ID, 3),
    });
    assert_eq!(
        world.inventory.cursor_item,
        item_stack(VANILLA_ITEM_STONE_ID, 1)
    );
    assert_eq!(world.counters.cursor_item_updates_received, 1);

    assert!(world.close_local_container(0));
    assert!(world.creative_inventory_screen().is_none());
    world.apply_set_cursor_item(SetCursorItem {
        item: item_stack(VANILLA_ITEM_DIAMOND_ID, 3),
    });
    assert_eq!(
        world.inventory.cursor_item,
        item_stack(VANILLA_ITEM_DIAMOND_ID, 3)
    );
}

#[test]
fn inventory_menu_corrections_sync_remote_player_slot_and_hotbar_pop_time() {
    let mut world = world_with_instrument_registry();
    world.local_player.abilities = Some(creative_abilities());
    world.open_local_inventory_with_operator_tab(false).unwrap();
    assert_eq!(world.creative_remote_inventory_menu_slots().len(), 46);

    assert!(world.apply_container_set_slot(ContainerSetSlot {
        container_id: 0,
        state_id: 1,
        slot: 36,
        item: item_stack(VANILLA_ITEM_STONE_ID, 2),
    }));
    assert_eq!(world.creative_hotbar_pop_times()[0], 5);
    assert_eq!(
        world
            .creative_remote_inventory_menu_slots()
            .iter()
            .find(|slot| slot.slot == 36)
            .unwrap()
            .item,
        item_stack(VANILLA_ITEM_STONE_ID, 2)
    );
    assert_eq!(
        world
            .inventory
            .player_slots
            .iter()
            .find(|slot| slot.slot == 0)
            .unwrap()
            .item,
        item_stack(VANILLA_ITEM_STONE_ID, 2)
    );

    world.advance_creative_hotbar_pop_times(3);
    assert_eq!(world.creative_hotbar_pop_times()[0], 2);
    assert!(world.apply_container_set_slot(ContainerSetSlot {
        container_id: 0,
        state_id: 2,
        slot: 36,
        item: item_stack(VANILLA_ITEM_DIAMOND_ID, 1),
    }));
    assert_eq!(
        world.creative_hotbar_pop_times()[0],
        2,
        "a nonempty lower-count correction does not restart pop time"
    );
    assert!(world.apply_container_set_slot(ContainerSetSlot {
        container_id: 0,
        state_id: 3,
        slot: 36,
        item: item_stack(VANILLA_ITEM_DIAMOND_ID, 4),
    }));
    assert_eq!(world.creative_hotbar_pop_times()[0], 5);
}

#[test]
fn level_reset_clears_creative_screen_remote_slots_and_pop_time() {
    let mut world = world_with_instrument_registry();
    world.local_player.abilities = Some(creative_abilities());
    world.open_local_inventory_with_operator_tab(false).unwrap();
    world.apply_container_set_slot(ContainerSetSlot {
        container_id: 0,
        state_id: 1,
        slot: 36,
        item: item_stack(VANILLA_ITEM_STONE_ID, 2),
    });

    world.clear_client_level();

    assert!(world.creative_inventory_screen().is_none());
    assert!(world.creative_remote_inventory_menu_slots().is_empty());
    assert_eq!(world.creative_hotbar_pop_times(), [0; 9]);
}

#[test]
fn registry_session_reset_clears_creative_interaction_state() {
    let mut world = world_with_instrument_registry();
    world.local_player.abilities = Some(creative_abilities());
    world.open_local_inventory_with_operator_tab(false).unwrap();
    world.apply_container_set_slot(ContainerSetSlot {
        container_id: 0,
        state_id: 1,
        slot: 36,
        item: item_stack(VANILLA_ITEM_STONE_ID, 2),
    });

    world.reset_registry_session();

    assert!(world.creative_inventory_screen().is_none());
    assert!(world.creative_remote_inventory_menu_slots().is_empty());
    assert_eq!(world.creative_hotbar_pop_times(), [0; 9]);
}

#[test]
fn every_materialized_stack_uses_a_known_protocol_item_id() {
    let mut world = world_with_instrument_registry();
    world.rebuild_creative_inventory(false).unwrap();
    for tab in &world.creative_inventory_snapshot().unwrap().tabs {
        assert!(tab
            .icon
            .item_id
            .is_some_and(|id| vanilla_item_type(id).is_some()));
        assert!(tab.items.iter().all(|stack| stack
            .item_id
            .is_some_and(|id| vanilla_item_type(id).is_some())));
    }
}
