use super::*;
use crate::entities::{
    vanilla_entity_metadata_baby_accessor_for_entity_type, VANILLA_ENTITY_TYPE_DONKEY_ID,
    VANILLA_ENTITY_TYPE_HORSE_ID, VANILLA_ENTITY_TYPE_LLAMA_ID, VANILLA_ENTITY_TYPE_NAUTILUS_ID,
};
use crate::{LevelEventSoundRandomState, LocalInteractionSoundEffect};
use bbb_protocol::builtin_enum_component_values::{
    VANILLA_DYE_COLOR_RED_ID, VANILLA_DYE_COLOR_YELLOW_ID,
};
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID, VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
    VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
    VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID, VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
    VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
    VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
    VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
    VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID,
};
use bbb_protocol::item_types::VANILLA_ITEM_ELYTRA_ID;
use bbb_protocol::menu_types::{
    VANILLA_MENU_TYPES, VANILLA_MENU_TYPE_GENERIC_9X2_ID, VANILLA_MENU_TYPE_GENERIC_9X3_ID,
    VANILLA_MENU_TYPE_LECTERN_ID,
};
use bbb_protocol::packets::{
    AddEntity as ProtocolAddEntity, CraftingRecipeDisplaySummary,
    EntityDataValue as ProtocolEntityDataValue, EntityDataValueKind, IngredientSummary,
    MountScreenOpen as ProtocolMountScreenOpen, PlayerAbilities as ProtocolPlayerAbilities,
    PlayerExperience as ProtocolPlayerExperience, RecipeBookAdd as ProtocolRecipeBookAdd,
    RecipeBookAddEntry as ProtocolRecipeBookAddEntry,
    RecipeDisplayEntry as ProtocolRecipeDisplayEntry, RecipeDisplayId, RecipeDisplaySummary,
    RecipeDisplayType, RecipePropertySetSummary, RegistryTags, ResolvableProfileKindSummary,
    ResolvableProfileSummary, SetEntityData as ProtocolSetEntityData, SlotDisplaySummary,
    StonecutterSelectableRecipeSummary, TagNetworkPayload, UpdateRecipes as ProtocolUpdateRecipes,
    UpdateTags as ProtocolUpdateTags, UseEffectsSummary as ProtocolUseEffectsSummary,
    Vec3d as ProtocolVec3d,
};
use bbb_protocol::recipe_wire_types::{
    VANILLA_RECIPE_BOOK_CATEGORY_STONECUTTER_ID, VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
    VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,
};
use uuid::Uuid;

mod fixtures;
use fixtures::*;

mod container_clicks;
mod crafting;
mod furnace_and_generic_menus;
mod item_components;
mod merchant;
mod mounts;
mod pattern_menus;
mod smithing_and_anvil;
mod state_and_hashing;

fn open_test_container(store: &mut WorldStore, container_id: i32) {
    store.apply_open_screen(ProtocolOpenScreen {
        container_id,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
}

fn component_rich_item_stack() -> ProtocolItemStackSummary {
    ProtocolItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
        count: 3,
        component_patch: ProtocolDataComponentPatchSummary {
            added_type_ids: vec![
                VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
                VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
            ],
            max_stack_size: Some(16),
            lock_predicate: Some(bbb_protocol::packets::PersistentItemPredicateSummary {
                count_min: Some(1),
                count_max: Some(3),
                ..Default::default()
            }),
            ..Default::default()
        }
        .into(),
    }
}

#[test]
fn component_rich_stacks_survive_inventory_equipment_and_pick_response_paths() {
    let mut store = WorldStore::new();
    let stack = component_rich_item_stack();

    assert!(
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 5,
            item: stack.clone(),
        })
    );
    assert!(store.apply_held_slot(bbb_protocol::packets::SetHeldSlot { slot: 5 }));
    assert_eq!(
        store.local_item_in_hand(InteractionHand::MainHand),
        Some(&stack)
    );
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: stack.clone(),
    });
    assert_eq!(store.inventory().cursor_item, stack);

    open_test_container(&mut store, 7);
    assert!(
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![stack.clone()],
            carried_item: stack.clone(),
        })
    );
    assert_eq!(
        store.inventory().open_container.as_ref().unwrap().slots[0].item,
        stack
    );
    assert_eq!(store.inventory().cursor_item, stack);

    let mut slot_stack = stack.clone();
    slot_stack.count = 2;
    slot_stack
        .component_patch
        .lock_predicate
        .as_mut()
        .unwrap()
        .count_max = Some(2);
    assert!(store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 7,
        state_id: 13,
        slot: 0,
        item: slot_stack.clone(),
    }));
    assert_eq!(
        store.inventory().open_container.as_ref().unwrap().slots[0].item,
        slot_stack
    );

    store.apply_add_entity(protocol_add_entity_with_type(
        123,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    ));
    assert!(
        store.apply_set_equipment(bbb_protocol::packets::SetEquipment {
            entity_id: 123,
            slots: vec![bbb_protocol::packets::EquipmentSlotUpdate {
                slot: EquipmentSlot::Head,
                item: stack.clone(),
            }],
        })
    );

    assert_eq!(
        store.local_item_in_hand(InteractionHand::MainHand),
        Some(&stack)
    );
    assert_eq!(store.inventory().cursor_item, stack);
    assert_eq!(
        store.inventory().open_container.as_ref().unwrap().slots[0].item,
        slot_stack
    );
    assert_eq!(store.equipment_item(123, EquipmentSlot::Head), Some(&stack));
}

#[test]
fn tracks_player_inventory_and_container_state() {
    let mut store = WorldStore::new();

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 36,
        item: item_stack(42, 1),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 36,
        item: item_stack(43, 2),
    });
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(99, 1),
    });

    assert_eq!(
        store.inventory().player_slots,
        vec![InventorySlot {
            slot: 36,
            item: item_stack(43, 2),
            local_selected_bundle_item_index: NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX,
        }]
    );
    assert_eq!(store.inventory().cursor_item, item_stack(99, 1));

    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![ProtocolItemStackSummary::empty(), item_stack(42, 64)],
        carried_item: ProtocolItemStackSummary::empty(),
    });
    store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 7,
        state_id: 13,
        slot: 1,
        item: item_stack(44, 3),
    });
    store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 7,
        id: 2,
        value: 9,
    });
    store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 7,
        id: 2,
        value: 10,
    });

    let container = store.inventory().open_container.as_ref().unwrap();
    assert_eq!(container.container_id, 7);
    assert_eq!(
        container.menu_type_id,
        Some(VANILLA_MENU_TYPE_GENERIC_9X3_ID)
    );
    assert_eq!(
        container
            .title
            .as_ref()
            .map(bbb_protocol::RichTextComponent::text),
        Some("Chest")
    );
    assert_eq!(container.state_id, 13);
    assert_eq!(
        container.slots,
        vec![
            ContainerSlot {
                slot: 0,
                item: ProtocolItemStackSummary::empty(),
                local_selected_bundle_item_index: NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX,
            },
            ContainerSlot {
                slot: 1,
                item: item_stack(44, 3),
                local_selected_bundle_item_index: NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX,
            },
        ]
    );
    assert_eq!(
        container.data_values,
        vec![ContainerDataValue { id: 2, value: 10 }]
    );
    assert_eq!(store.open_container_data_value(2), Some(10));
    assert_eq!(store.open_container_data_value(3), None);
    assert_eq!(
        store.inventory().cursor_item,
        ProtocolItemStackSummary::empty()
    );

    assert!(store.apply_container_close(ProtocolContainerClose { container_id: 99 }));
    assert!(store.inventory().open_container.is_none());
    assert_eq!(store.open_container_data_value(2), None);
    assert!(!store.apply_container_close(ProtocolContainerClose { container_id: 7 }));

    assert_eq!(store.counters().inventory_slot_updates_received, 2);
    assert_eq!(store.counters().inventory_slots_tracked, 1);
    assert_eq!(store.counters().cursor_item_updates_received, 1);
    assert_eq!(store.counters().container_open_updates_received, 1);
    assert_eq!(store.counters().container_content_updates_received, 1);
    assert_eq!(store.counters().container_slot_updates_received, 1);
    assert_eq!(store.counters().container_data_updates_received, 2);
    assert_eq!(store.counters().container_close_updates_received, 2);
    assert_eq!(store.counters().container_close_updates_applied, 1);
    assert_eq!(store.counters().container_close_updates_ignored, 1);
}

#[test]
fn set_player_inventory_only_tracks_vanilla_inventory_and_equipment_slots() {
    let mut store = WorldStore::new();

    assert!(
        !store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: -1,
            item: item_stack(20, 1),
        })
    );
    assert!(
        !store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 43,
            item: item_stack(21, 1),
        })
    );
    assert!(store.inventory().player_slots.is_empty());

    assert!(
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: VANILLA_PLAYER_INVENTORY_SADDLE_SLOT,
            item: item_stack(22, 1),
        })
    );
    assert_eq!(
        player_slot_item(&store, VANILLA_PLAYER_INVENTORY_SADDLE_SLOT),
        item_stack(22, 1)
    );
    assert!(store.inventory().inventory_menu.slots.is_empty());

    assert!(
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(23, 2),
        })
    );
    assert_eq!(player_slot_item(&store, 0), item_stack(23, 2));
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
        item_stack(23, 2)
    );
    assert_eq!(store.counters().inventory_slot_updates_received, 4);
    assert_eq!(store.counters().inventory_slots_tracked, 2);
}

#[test]
fn container_set_data_only_updates_the_matching_active_menu() {
    let mut store = WorldStore::new();

    assert!(!store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 7,
        id: 2,
        value: 9,
    }));
    assert!(store.inventory().open_container.is_none());

    assert!(store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 0,
        id: 1,
        value: 3,
    }));
    assert_eq!(
        store.inventory().inventory_menu.data_values,
        vec![ContainerDataValue { id: 1, value: 3 }]
    );

    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    assert!(!store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 8,
        id: 2,
        value: 10,
    }));
    let container = store.inventory().open_container.as_ref().unwrap();
    assert_eq!(container.container_id, 7);
    assert!(container.data_values.is_empty());

    assert!(store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 7,
        id: 2,
        value: 11,
    }));
    assert_eq!(store.open_container_data_value(2), Some(11));
    assert_eq!(store.counters().container_data_updates_received, 4);
}

#[test]
fn container_packets_reject_out_of_range_slots_and_data_ids() {
    let mut store = WorldStore::new();
    open_test_container(&mut store, 7);

    // GENERIC_9X3 has 27 + 36 = 63 slots (0..63); out-of-range slots must be ignored.
    assert!(!store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 7,
        state_id: 1,
        slot: -1,
        item: item_stack(10, 1),
    }));
    assert!(!store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 7,
        state_id: 2,
        slot: 63,
        item: item_stack(10, 1),
    }));
    assert!(store
        .inventory()
        .open_container
        .as_ref()
        .unwrap()
        .slots
        .is_empty());

    // Data slot ids outside 0..16 must be rejected.
    assert!(!store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 7,
        id: -1,
        value: 5,
    }));
    assert!(!store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 7,
        id: 16,
        value: 5,
    }));
    assert!(store
        .inventory()
        .open_container
        .as_ref()
        .unwrap()
        .data_values
        .is_empty());
}

#[test]
fn container_content_only_updates_the_matching_active_menu() {
    let mut store = WorldStore::new();
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(10, 1),
    });
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });

    assert!(
        !store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 8,
            state_id: 4,
            items: vec![item_stack(20, 2)],
            carried_item: item_stack(30, 3),
        })
    );
    let container = store.inventory().open_container.as_ref().unwrap();
    assert_eq!(container.container_id, 7);
    assert_eq!(container.state_id, 0);
    assert!(container.slots.is_empty());
    assert_eq!(store.inventory().cursor_item, item_stack(10, 1));

    assert!(
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 5,
            items: vec![item_stack(21, 2)],
            carried_item: item_stack(31, 3),
        })
    );
    let container = store.inventory().open_container.as_ref().unwrap();
    assert_eq!(container.container_id, 7);
    assert_eq!(container.state_id, 5);
    assert_eq!(container.slots[0].item, item_stack(21, 2));
    assert_eq!(store.inventory().cursor_item, item_stack(31, 3));
    assert_eq!(store.counters().container_content_updates_received, 2);
}

#[test]
fn container_set_slot_only_updates_inventory_or_the_matching_active_menu() {
    let mut store = WorldStore::new();
    open_test_container(&mut store, 7);

    assert!(!store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 8,
        state_id: 4,
        slot: 1,
        item: item_stack(20, 2),
    }));
    let container = store.inventory().open_container.as_ref().unwrap();
    assert_eq!(container.container_id, 7);
    assert_eq!(container.state_id, 0);
    assert!(container.slots.is_empty());

    assert!(store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 0,
        state_id: 5,
        slot: INVENTORY_MENU_HOTBAR_START,
        item: item_stack(21, 2),
    }));
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
        item_stack(21, 2)
    );

    assert!(store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 7,
        state_id: 6,
        slot: 1,
        item: item_stack(22, 3),
    }));
    let container = store.inventory().open_container.as_ref().unwrap();
    assert_eq!(container.state_id, 6);
    assert_eq!(container.slots[0].item, item_stack(22, 3));
    assert_eq!(store.counters().container_slot_updates_received, 3);
}

#[test]
fn mount_equipment_visibility_uses_tame_and_baby_entity_data() {
    let horse_accessors = vanilla_entity_metadata_abstract_horse_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_DONKEY_ID,
    )
    .expect("generated AbstractHorse metadata accessors must resolve");
    let (baby_data_id, baby_default) =
        vanilla_entity_metadata_baby_accessor_for_entity_type(VANILLA_ENTITY_TYPE_DONKEY_ID)
            .expect("generated Donkey inherited baby accessor must resolve");
    assert!(
        !baby_default,
        "vanilla AgeableMob baby default must be false"
    );
    assert_eq!(
        mount_visibility_for_entity(
            VANILLA_ENTITY_TYPE_DONKEY_ID,
            vec![protocol_byte_data(
                horse_accessors.flags_data_id,
                horse_accessors.tame_mask,
            )],
        ),
        Some(MountEquipmentSlotVisibility {
            saddle: true,
            body: None,
        })
    );
    assert_eq!(
        mount_visibility_for_entity(
            VANILLA_ENTITY_TYPE_DONKEY_ID,
            vec![
                protocol_bool_data(baby_data_id, true),
                protocol_byte_data(horse_accessors.flags_data_id, horse_accessors.tame_mask,),
            ],
        ),
        Some(MountEquipmentSlotVisibility {
            saddle: false,
            body: None,
        })
    );
    assert_eq!(
        mount_visibility_for_entity(
            VANILLA_ENTITY_TYPE_NAUTILUS_ID,
            vec![{
                let accessors = vanilla_entity_metadata_tamable_animal_accessors_for_entity_type(
                    VANILLA_ENTITY_TYPE_NAUTILUS_ID,
                )
                .expect("generated TamableAnimal metadata accessors must resolve");
                protocol_byte_data(accessors.flags_data_id, accessors.tame_mask)
            }],
        ),
        Some(MountEquipmentSlotVisibility {
            saddle: true,
            body: Some(MountArmorSlotKind::Nautilus),
        })
    );
    assert_eq!(
        mount_visibility_for_entity(VANILLA_ENTITY_TYPE_NAUTILUS_ID, Vec::new()),
        Some(MountEquipmentSlotVisibility {
            saddle: false,
            body: None,
        })
    );
}

#[test]
fn apply_local_mount_quick_move_moves_mount_slots_to_player_reverse() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        42,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    store.apply_mount_screen_open(ProtocolMountScreenOpen {
        container_id: 7,
        inventory_columns: 5,
        entity_id: 42,
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 53];
    items[MOUNT_INVENTORY_START as usize] = item_stack(43, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MOUNT_INVENTORY_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (MOUNT_INVENTORY_START, ProtocolHashedStack::Empty),
            (52, hashed_item_stack(43, 3)),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, MOUNT_INVENTORY_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(open_container_slot_item(&store, 52), item_stack(43, 3));
}

#[test]
fn marker_particle_target_requires_creative_and_main_hand_barrier_or_light() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(VANILLA_ITEM_BARRIER_ID, 1),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 1,
        item: item_stack(VANILLA_ITEM_LIGHT_ID, 1),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 2,
        item: item_stack(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID, 1),
    });

    assert_eq!(store.local_marker_particle_target_block(), None);

    store.gameplay.game_type = bbb_protocol::game_types::VANILLA_GAME_TYPE_CREATIVE_ID;
    assert_eq!(
        store.local_marker_particle_target_block(),
        Some("minecraft:barrier")
    );

    assert!(store.set_local_selected_hotbar_slot(1));
    assert_eq!(
        store.local_marker_particle_target_block(),
        Some("minecraft:light")
    );

    assert!(store.set_local_selected_hotbar_slot(2));
    assert_eq!(store.local_marker_particle_target_block(), None);

    assert!(store.set_local_selected_hotbar_slot(3));
    assert_eq!(store.local_marker_particle_target_block(), None);
}

#[test]
fn local_selected_main_hand_attack_range_reads_default_and_patch_components() {
    let mut store = WorldStore::new();
    let default_range = item_attack_range(2.0, 4.5);
    let patch_range = ProtocolAttackRangeSummary {
        min_reach: 1.0,
        max_reach: 2.25,
        min_creative_reach: 1.0,
        max_creative_reach: 3.0,
        hitbox_margin: 0.25,
        mob_factor: 0.75,
    };
    store.items.commit_item_component_defaults(BTreeMap::from([(
        42,
        ProtocolDataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID],
            attack_range: Some(default_range),
            ..Default::default()
        },
    )]));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 1),
    });

    assert_eq!(
        store.local_selected_main_hand_attack_range(),
        Some(default_range)
    );

    let mut patched = item_stack(99, 1);
    patched.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID];
    patched.component_patch.attack_range = Some(patch_range);
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: patched,
    });

    assert_eq!(
        store.local_selected_main_hand_attack_range(),
        Some(ItemAttackRange {
            min_reach: 1.0,
            max_reach: 2.25,
            min_creative_reach: 1.0,
            max_creative_reach: 3.0,
            hitbox_margin: 0.25,
            mob_factor: 0.75,
        })
    );
}

#[test]
fn effective_food_properties_apply_default_patch_and_removal_precedence() {
    let mut store = WorldStore::new();
    store.items.commit_item_component_defaults(BTreeMap::from([(
        42,
        ProtocolDataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_FOOD_TYPE_ID],
            food_nutrition: Some(4),
            food_saturation_bits: Some(2.4_f32.to_bits()),
            food_can_always_eat: false,
            ..Default::default()
        },
    )]));

    let default_stack = item_stack(42, 1);
    let effective_default = store.effective_item_stack(&default_stack);
    assert_eq!(effective_default.component_patch.food_nutrition, Some(4));
    assert_eq!(
        effective_default.component_patch.food_saturation_bits,
        Some(2.4_f32.to_bits())
    );
    assert!(!effective_default.component_patch.food_can_always_eat);

    let mut patched = item_stack(42, 1);
    patched.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_FOOD_TYPE_ID];
    patched.component_patch.food_nutrition = Some(6);
    patched.component_patch.food_saturation_bits = Some(7.2_f32.to_bits());
    patched.component_patch.food_can_always_eat = true;
    let effective_patch = store.effective_item_stack(&patched);
    assert_eq!(effective_patch.component_patch.food_nutrition, Some(6));
    assert_eq!(
        effective_patch.component_patch.food_saturation_bits,
        Some(7.2_f32.to_bits())
    );
    assert!(effective_patch.component_patch.food_can_always_eat);

    patched.component_patch.removed_type_ids = vec![VANILLA_DATA_COMPONENT_FOOD_TYPE_ID];
    let effective_removed = store.effective_item_stack(&patched);
    assert!(effective_removed
        .component_patch
        .removed_type_ids
        .contains(&VANILLA_DATA_COMPONENT_FOOD_TYPE_ID));
    assert!(
        !store.item_stack_has_effective_component(&patched, VANILLA_DATA_COMPONENT_FOOD_TYPE_ID)
    );
}

#[test]
fn local_food_consumption_gate_matches_player_can_eat_and_component_precedence() {
    let mut store = WorldStore::new();
    store.set_default_item_component_codec_values(BTreeMap::from([(
        42,
        BTreeMap::from([(
            VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
            serde_json::json!({}),
        )]),
    )]));
    store.items.commit_item_component_defaults(BTreeMap::from([(
        42,
        ProtocolDataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_FOOD_TYPE_ID],
            food_nutrition: Some(4),
            food_saturation_bits: Some(2.4_f32.to_bits()),
            food_can_always_eat: false,
            ..Default::default()
        },
    )]));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 1),
    });

    assert!(!store.local_item_in_hand_can_start_consuming(InteractionHand::MainHand));

    store.apply_player_health(bbb_protocol::packets::PlayerHealth {
        health: 20.0,
        food: 19,
        saturation: 5.0,
    });
    assert!(store.local_item_in_hand_can_start_consuming(InteractionHand::MainHand));

    store.apply_player_health(bbb_protocol::packets::PlayerHealth {
        health: 20.0,
        food: 20,
        saturation: 5.0,
    });
    let mut always_edible = item_stack(42, 1);
    always_edible.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_FOOD_TYPE_ID];
    always_edible.component_patch.food_nutrition = Some(1);
    always_edible.component_patch.food_saturation_bits = Some(0.0_f32.to_bits());
    always_edible.component_patch.food_can_always_eat = true;
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: always_edible,
    });
    assert!(store.local_item_in_hand_can_start_consuming(InteractionHand::MainHand));

    let mut removed_food = item_stack(42, 1);
    removed_food.component_patch.removed_type_ids = vec![VANILLA_DATA_COMPONENT_FOOD_TYPE_ID];
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: removed_food,
    });
    assert!(store.local_item_in_hand_can_start_consuming(InteractionHand::MainHand));

    let mut removed_consumable = item_stack(42, 1);
    removed_consumable.component_patch.removed_type_ids =
        vec![VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID];
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: removed_consumable,
    });
    assert!(store.local_item_in_hand_can_start_consuming(InteractionHand::MainHand));

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 1),
    });
    store.apply_player_abilities(ProtocolPlayerAbilities {
        invulnerable: true,
        flying: false,
        can_fly: true,
        instabuild: true,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
    assert!(store.local_item_in_hand_can_start_consuming(InteractionHand::MainHand));
}

#[test]
fn local_selected_main_hand_attack_range_removed_component_overrides_default_and_added() {
    let mut store = WorldStore::new();
    store.items.commit_item_component_defaults(BTreeMap::from([(
        42,
        ProtocolDataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID],
            attack_range: Some(item_attack_range(2.0, 4.5)),
            ..Default::default()
        },
    )]));
    let mut item = item_stack(42, 1);
    item.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID];
    item.component_patch.attack_range = Some(ProtocolAttackRangeSummary {
        min_reach: 1.0,
        max_reach: 2.25,
        min_creative_reach: 1.0,
        max_creative_reach: 3.0,
        hitbox_margin: 0.25,
        mob_factor: 0.75,
    });
    item.component_patch.removed_type_ids = vec![VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID];
    store.apply_set_player_inventory(ProtocolSetPlayerInventory { slot: 0, item });

    assert_eq!(store.local_selected_main_hand_attack_range(), None);
}

#[test]
fn local_attack_minimum_charge_applies_default_patch_and_removal_precedence() {
    let mut store = WorldStore::new();
    store.items.commit_item_component_defaults(BTreeMap::from([(
        42,
        ProtocolDataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID],
            minimum_attack_charge_bits: Some(1.0_f32.to_bits()),
            ..Default::default()
        },
    )]));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 1),
    });
    assert_eq!(
        store
            .local_selected_main_hand_minimum_attack_charge()
            .to_bits(),
        1.0_f32.to_bits()
    );

    let mut patched = item_stack(42, 1);
    patched.component_patch.added_type_ids =
        vec![VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID];
    patched.component_patch.minimum_attack_charge_bits = Some(0.5_f32.to_bits());
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: patched.clone(),
    });
    assert_eq!(
        store
            .local_selected_main_hand_minimum_attack_charge()
            .to_bits(),
        0.5_f32.to_bits()
    );

    patched.component_patch.removed_type_ids =
        vec![VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID];
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: patched,
    });
    assert_eq!(store.local_selected_main_hand_minimum_attack_charge(), 0.0);
}

#[test]
fn local_using_item_use_effects_reads_default_and_patch_components() {
    let mut store = WorldStore::new();
    let default_effects = ItemUseEffects {
        can_sprint: true,
        interact_vibrations: false,
        speed_multiplier: 1.0,
    };
    store.items.commit_item_component_defaults(BTreeMap::from([(
        42,
        ProtocolDataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID],
            use_effects: Some(ProtocolUseEffectsSummary {
                can_sprint: default_effects.can_sprint,
                interact_vibrations: default_effects.interact_vibrations,
                speed_multiplier: default_effects.speed_multiplier,
            }),
            ..Default::default()
        },
    )]));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 1),
    });
    store.set_local_using_item(true);

    assert_eq!(store.local_using_item_use_effects(), Some(default_effects));

    let mut patched = item_stack(99, 1);
    patched.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID];
    patched.component_patch.use_effects = Some(ProtocolUseEffectsSummary {
        can_sprint: false,
        interact_vibrations: true,
        speed_multiplier: 0.5,
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: patched,
    });

    assert_eq!(
        store.local_using_item_use_effects(),
        Some(ItemUseEffects {
            can_sprint: false,
            interact_vibrations: true,
            speed_multiplier: 0.5,
        })
    );
}

#[test]
fn local_player_usable_glider_applies_default_components_slot_and_damage() {
    let mut store = WorldStore::new();
    store.set_default_item_component_codec_values(BTreeMap::from([(
        42,
        BTreeMap::from([(VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID, serde_json::json!({}))]),
    )]));
    store.items.commit_item_component_defaults(BTreeMap::from([(
        42,
        ProtocolDataComponentPatchSummary {
            added_type_ids: vec![
                VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
                VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
                VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
                VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
            ],
            max_stack_size: Some(1),
            max_damage: Some(432),
            damage: Some(0),
            equippable: Some(crate::item_profiles::test_equippable(EquipmentSlot::Chest)),
            ..Default::default()
        },
    )]));

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_CHEST_SLOT,
        item: item_stack(42, 1),
    });
    assert!(store.local_player_has_usable_glider());

    let mut damaged = item_stack(42, 1);
    damaged.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID];
    damaged.component_patch.damage = Some(430);
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_CHEST_SLOT,
        item: damaged.clone(),
    });
    assert!(store.local_player_has_usable_glider());

    damaged.component_patch.damage = Some(431);
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_CHEST_SLOT,
        item: damaged.clone(),
    });
    assert!(!store.local_player_has_usable_glider());
    damaged
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID);
    damaged.component_patch.unbreakable = true;
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_CHEST_SLOT,
        item: damaged,
    });
    assert!(store.local_player_has_usable_glider());
}

#[test]
fn local_player_usable_glider_honors_equippable_patch_slot_and_removals() {
    fn equippable(slot_id: i32) -> bbb_protocol::packets::EquippableSummary {
        let sound = || bbb_protocol::packets::SoundEventSummary {
            registry_id: Some(0),
            sound_id: None,
            fixed_range_bits: None,
        };
        bbb_protocol::packets::EquippableSummary {
            slot_id,
            equip_sound: sound(),
            asset_id: None,
            camera_overlay: None,
            allowed_entities: None,
            dispensable: true,
            swappable: true,
            damage_on_hurt: true,
            equip_on_interact: false,
            can_be_sheared: false,
            shearing_sound: sound(),
        }
    }

    let mut store = WorldStore::new();
    store.set_default_item_component_codec_values(BTreeMap::from([(
        43,
        BTreeMap::from([(VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID, serde_json::json!({}))]),
    )]));
    store.set_test_equippable_defaults([(
        43,
        crate::item_profiles::test_equippable(EquipmentSlot::Chest),
    )]);

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: item_stack(43, 1),
    });
    assert!(!store.local_player_has_usable_glider());

    let mut offhand = item_stack(43, 1);
    offhand.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID];
    offhand.component_patch.equippable = Some(equippable(
        bbb_protocol::equipment_slots::VANILLA_EQUIPMENT_SLOT_OFFHAND_ID,
    ));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: offhand.clone(),
    });
    assert!(store.local_player_has_usable_glider());

    offhand.component_patch.removed_type_ids = vec![VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID];
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: offhand,
    });
    assert!(!store.local_player_has_usable_glider());
}

#[test]
fn local_bundle_selection_tracks_player_inventory_slot() {
    let mut store = WorldStore::new();

    assert!(!store.apply_local_select_bundle_item(4, 0));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 4,
        item: bundle_item_stack(42, 1, 3),
    });

    assert!(!store.apply_local_select_bundle_item(4, -2));
    assert_eq!(
        player_slot_selection(&store, 4),
        NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
    );

    assert!(store.apply_local_select_bundle_item(4, 1));
    assert_eq!(player_slot_selection(&store, 4), 1);

    assert!(store.apply_local_select_bundle_item(4, -1));
    assert_eq!(
        player_slot_selection(&store, 4),
        NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
    );

    assert!(store.apply_local_select_bundle_item(4, 2));
    assert_eq!(player_slot_selection(&store, 4), 2);

    assert!(store.apply_local_select_bundle_item(4, 2));
    assert_eq!(
        player_slot_selection(&store, 4),
        NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
    );

    assert!(store.apply_local_select_bundle_item(4, 0));
    assert_eq!(player_slot_selection(&store, 4), 0);

    assert!(store.apply_local_select_bundle_item(4, 3));
    assert_eq!(
        player_slot_selection(&store, 4),
        NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
    );

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 5,
        item: item_stack(43, 1),
    });
    assert!(!store.apply_local_select_bundle_item(5, 0));
    assert_eq!(
        player_slot_selection(&store, 5),
        NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
    );
}

#[test]
fn local_container_close_does_not_count_clientbound_close_packet() {
    let mut store = WorldStore::new();

    assert!(store.open_local_inventory());
    assert!(store.close_local_container(INVENTORY_MENU_CONTAINER_ID));
    assert!(!store.local_inventory_is_open());
    assert_eq!(store.open_container_id(), None);
    assert_eq!(store.counters().container_close_updates_received, 0);
    assert!(!store.close_local_container(INVENTORY_MENU_CONTAINER_ID));

    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });

    assert!(store.close_local_container(7));
    assert!(store.inventory().open_container.is_none());
    assert_eq!(store.counters().container_close_updates_received, 0);
    assert_eq!(store.counters().container_close_updates_applied, 0);
    assert_eq!(store.counters().container_close_updates_ignored, 0);

    assert!(!store.close_local_container(7));
    assert_eq!(store.counters().container_close_updates_received, 0);
    assert_eq!(store.counters().container_close_updates_applied, 0);
    assert_eq!(store.counters().container_close_updates_ignored, 0);
}

#[test]
fn merchant_offers_apply_only_to_matching_open_container() {
    let mut store = WorldStore::new();

    assert!(!store.apply_merchant_offers(merchant_offers(7, 1)));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    assert!(!store.apply_merchant_offers(merchant_offers(7, 1)));
    assert!(store
        .inventory()
        .open_container
        .as_ref()
        .unwrap()
        .merchant_offers
        .is_none());
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    assert!(!store.apply_merchant_offers(merchant_offers(99, 1)));
    assert!(store.apply_merchant_offers(merchant_offers(7, 2)));

    let container = store.inventory().open_container.as_ref().unwrap();
    let offers = container.merchant_offers.as_ref().unwrap();
    assert_eq!(offers.container_id, 7);
    assert_eq!(offers.offers.len(), 2);
    assert_eq!(offers.villager_level, 3);
    assert_eq!(offers.villager_xp, 120);
    assert!(offers.show_progress);
    assert!(!offers.can_restock);
    assert_eq!(offers.local_selected_offer_index, 0);
    assert_eq!(offers.local_scroll_offset, 0);
    assert_eq!(offers.offers[0].buy_a, item_cost(42, 3));
    assert_eq!(offers.offers[0].sell, item_stack(99, 1));
    assert!(store.set_local_merchant_selected_offer(1));
    assert!(!store.set_local_merchant_selected_offer(2));
    assert!(!store.set_local_merchant_selected_offer(-1));
    assert_eq!(
        store
            .inventory()
            .open_container
            .as_ref()
            .and_then(|container| container.merchant_offers.as_ref())
            .map(|offers| offers.local_selected_offer_index),
        Some(1)
    );

    assert_eq!(store.counters().merchant_offer_packets_received, 4);
    assert_eq!(store.counters().merchant_offer_packets_applied, 1);
    assert_eq!(store.counters().merchant_offer_packets_ignored, 3);
    assert_eq!(store.counters().merchant_offers_tracked, 2);

    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 5,
        items: Vec::new(),
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert_eq!(store.counters().merchant_offers_tracked, 2);

    assert!(store.apply_container_close(ProtocolContainerClose { container_id: 7 }));
    assert_eq!(store.counters().container_close_updates_applied, 1);
    assert_eq!(store.counters().container_close_updates_ignored, 0);
    assert_eq!(store.counters().merchant_offers_tracked, 0);
}

#[test]
fn merchant_offer_replacement_retains_exact_cost_components_and_resets_local_view() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    assert!(store.apply_merchant_offers(merchant_offers(7, 8)));
    assert!(store.set_local_merchant_selected_offer(4));
    assert!(store.scroll_local_merchant_offers(1));

    let mut replacement = merchant_offers(7, 1);
    let components = &mut replacement.offers[0].buy_a.component_predicate.components;
    components.added_type_ids = vec![VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID];
    components.item_model = Some("minecraft:emerald".to_string());
    assert!(store.apply_merchant_offers(replacement));

    let offers = store
        .inventory()
        .open_container
        .as_ref()
        .and_then(|container| container.merchant_offers.as_ref())
        .unwrap();
    assert_eq!(offers.local_selected_offer_index, 0);
    assert_eq!(offers.local_scroll_offset, 0);
    assert_eq!(
        offers.offers[0]
            .buy_a
            .component_predicate
            .components
            .added_type_ids,
        vec![VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID]
    );
    assert_eq!(
        offers.offers[0]
            .buy_a
            .component_predicate
            .components
            .item_model
            .as_deref(),
        Some("minecraft:emerald")
    );
}

#[test]
fn apply_local_container_click_slot_picks_up_and_places_stack() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 3),
    });
    assert!(store.open_local_inventory());

    let pickup = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(pickup.container_id, INVENTORY_MENU_CONTAINER_ID);
    assert_eq!(pickup.slot_num, INVENTORY_MENU_HOTBAR_START);
    assert_eq!(
        pickup.changed_slots,
        BTreeMap::from([(INVENTORY_MENU_HOTBAR_START, ProtocolHashedStack::Empty)])
    );
    assert_eq!(
        pickup.carried_item,
        ProtocolHashedStack::Item(ProtocolHashedItemStack {
            item_id: 42,
            count: 3,
            components: ProtocolHashedComponentPatch::default(),
        })
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        player_slot_item(&store, 0),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(store.inventory().cursor_item, item_stack(42, 3));

    let place = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 10,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(
        place.changed_slots,
        BTreeMap::from([(
            10,
            ProtocolHashedStack::Item(ProtocolHashedItemStack {
                item_id: 42,
                count: 3,
                components: ProtocolHashedComponentPatch::default(),
            })
        )])
    );
    assert_eq!(place.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(inventory_menu_slot_item(&store, 10), item_stack(42, 3));
    assert_eq!(player_slot_item(&store, 10), item_stack(42, 3));
    assert_eq!(
        store.inventory().cursor_item,
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn local_container_slot_access_matches_specialized_input_predicates() {
    let mut store = WorldStore::new();
    store.set_furnace_fuel_item_ids(BTreeSet::from([43]));
    store.set_brewing_potion_item_ids(BTreeSet::from([44]));
    store.set_brewing_ingredient_item_ids(BTreeSet::from([45]));
    store.set_enchantment_lapis_lazuli_item_ids(BTreeSet::from([47]));
    store.set_cartography_additional_items(BTreeMap::from([(
        55,
        crate::CartographyAdditionalItemKind::Paper,
    )]));
    store.items.commit_item_component_defaults(BTreeMap::from([(
        48,
        ProtocolDataComponentPatchSummary {
            added_type_ids: vec![
                VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
                VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
                VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
            ],
            max_stack_size: Some(1),
            max_damage: Some(64),
            damage: Some(0),
            ..Default::default()
        },
    )]));
    store.apply_update_recipes(update_recipes(vec![
        (SMITHING_TEMPLATE_PROPERTY_SET, vec![52]),
        (SMITHING_BASE_PROPERTY_SET, vec![53]),
        (SMITHING_ADDITION_PROPERTY_SET, vec![54]),
    ]));
    apply_item_tags(
        &mut store,
        vec![
            (BEACON_PAYMENT_ITEM_TAG, vec![42]),
            (BREWING_STAND_FUEL_ITEM_TAG, vec![46]),
            (LOOM_BANNER_ITEM_TAG, vec![49]),
            (LOOM_DYE_ITEM_TAG, vec![50]),
            (LOOM_PATTERN_ITEM_TAG, vec![51]),
        ],
    );

    let plain = item_stack(41, 2);
    let beacon_payment = item_stack(42, 2);
    let furnace_fuel = item_stack(43, 2);
    let potion = item_stack(44, 2);
    let brewing_ingredient = item_stack(45, 2);
    let brewing_fuel = item_stack(46, 2);
    let lapis = item_stack(47, 2);
    let damageable = item_stack(48, 1);
    let banner = item_stack(49, 2);
    let dye = item_stack_with_component_summary(50, 2, VANILLA_DATA_COMPONENT_DYE_TYPE_ID);
    let pattern = item_stack_with_component_summary(
        51,
        2,
        VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
    );
    let pattern_without_component = item_stack(51, 2);
    let smithing_template = item_stack(52, 2);
    let smithing_base = item_stack(53, 2);
    let smithing_addition = item_stack(54, 2);
    let cartography_additional = item_stack(55, 2);
    let map = map_id_item_stack(56, 2, 7);
    let bucket = item_stack(VANILLA_ITEM_BUCKET_ID, 2);
    let ctx = InventoryCtx {
        items: &store.items,
        local_player: &store.local_player,
        entities: &store.entities,
        recipes: &store.recipes,
        registries: &store.registries,
    };
    let access = |menu_type_id, slot_num, stack: &ProtocolItemStackSummary| {
        local_container_slot_access(
            ctx,
            7,
            Some(menu_type_id),
            None,
            &[],
            slot_num,
            stack,
            stack,
        )
        .unwrap()
    };

    let mut unbreakable = damageable.clone();
    unbreakable.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID];
    unbreakable.component_patch.unbreakable = true;
    assert!(
        !access(
            VANILLA_MENU_TYPE_GRINDSTONE_ID,
            GRINDSTONE_INPUT_SLOT,
            &unbreakable
        )
        .may_place
    );
    for component_type_id in [
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
    ] {
        let mut removed = damageable.clone();
        removed.component_patch.removed_type_ids = vec![component_type_id];
        assert!(
            !access(
                VANILLA_MENU_TYPE_GRINDSTONE_ID,
                GRINDSTONE_INPUT_SLOT,
                &removed
            )
            .may_place
        );
    }

    for (menu_type_id, slot_num, stack, expected_max_stack_size) in [
        (
            VANILLA_MENU_TYPE_BEACON_ID,
            BEACON_PAYMENT_SLOT,
            &beacon_payment,
            1,
        ),
        (
            VANILLA_MENU_TYPE_FURNACE_ID,
            FURNACE_FUEL_SLOT,
            &furnace_fuel,
            VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE,
        ),
        (VANILLA_MENU_TYPE_FURNACE_ID, FURNACE_FUEL_SLOT, &bucket, 1),
        (
            VANILLA_MENU_TYPE_BREWING_STAND_ID,
            BREWING_STAND_BOTTLE_SLOT_START,
            &potion,
            1,
        ),
        (
            VANILLA_MENU_TYPE_BREWING_STAND_ID,
            BREWING_STAND_INGREDIENT_SLOT,
            &brewing_ingredient,
            VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE,
        ),
        (
            VANILLA_MENU_TYPE_BREWING_STAND_ID,
            BREWING_STAND_FUEL_SLOT,
            &brewing_fuel,
            VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE,
        ),
        (
            VANILLA_MENU_TYPE_ENCHANTMENT_ID,
            ENCHANTMENT_LAPIS_SLOT,
            &lapis,
            VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE,
        ),
        (
            VANILLA_MENU_TYPE_GRINDSTONE_ID,
            GRINDSTONE_INPUT_SLOT,
            &damageable,
            1,
        ),
        (
            VANILLA_MENU_TYPE_LOOM_ID,
            LOOM_BANNER_SLOT,
            &banner,
            VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE,
        ),
        (
            VANILLA_MENU_TYPE_LOOM_ID,
            LOOM_DYE_SLOT,
            &dye,
            VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE,
        ),
        (
            VANILLA_MENU_TYPE_LOOM_ID,
            LOOM_PATTERN_SLOT,
            &pattern,
            VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE,
        ),
        (
            VANILLA_MENU_TYPE_SMITHING_ID,
            SMITHING_TEMPLATE_SLOT,
            &smithing_template,
            VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE,
        ),
        (
            VANILLA_MENU_TYPE_SMITHING_ID,
            SMITHING_BASE_SLOT,
            &smithing_base,
            VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE,
        ),
        (
            VANILLA_MENU_TYPE_SMITHING_ID,
            SMITHING_ADDITIONAL_SLOT,
            &smithing_addition,
            VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE,
        ),
        (
            VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
            CARTOGRAPHY_TABLE_MAP_SLOT,
            &map,
            VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE,
        ),
        (
            VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
            CARTOGRAPHY_TABLE_ADDITIONAL_SLOT,
            &cartography_additional,
            VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE,
        ),
    ] {
        let access = access(menu_type_id, slot_num, stack);
        assert!(access.may_place, "menu {menu_type_id} slot {slot_num}");
        assert_eq!(
            access.max_stack_size, expected_max_stack_size,
            "menu {menu_type_id} slot {slot_num}"
        );
    }

    for (menu_type_id, slot_num, stack) in [
        (VANILLA_MENU_TYPE_BEACON_ID, BEACON_PAYMENT_SLOT, &plain),
        (VANILLA_MENU_TYPE_FURNACE_ID, FURNACE_FUEL_SLOT, &plain),
        (
            VANILLA_MENU_TYPE_BREWING_STAND_ID,
            BREWING_STAND_BOTTLE_SLOT_START,
            &plain,
        ),
        (
            VANILLA_MENU_TYPE_BREWING_STAND_ID,
            BREWING_STAND_INGREDIENT_SLOT,
            &plain,
        ),
        (
            VANILLA_MENU_TYPE_BREWING_STAND_ID,
            BREWING_STAND_FUEL_SLOT,
            &plain,
        ),
        (
            VANILLA_MENU_TYPE_ENCHANTMENT_ID,
            ENCHANTMENT_LAPIS_SLOT,
            &plain,
        ),
        (
            VANILLA_MENU_TYPE_GRINDSTONE_ID,
            GRINDSTONE_INPUT_SLOT,
            &plain,
        ),
        (VANILLA_MENU_TYPE_LOOM_ID, LOOM_BANNER_SLOT, &plain),
        (VANILLA_MENU_TYPE_LOOM_ID, LOOM_DYE_SLOT, &plain),
        (
            VANILLA_MENU_TYPE_LOOM_ID,
            LOOM_PATTERN_SLOT,
            &pattern_without_component,
        ),
        (
            VANILLA_MENU_TYPE_SMITHING_ID,
            SMITHING_TEMPLATE_SLOT,
            &plain,
        ),
        (VANILLA_MENU_TYPE_SMITHING_ID, SMITHING_BASE_SLOT, &plain),
        (
            VANILLA_MENU_TYPE_SMITHING_ID,
            SMITHING_ADDITIONAL_SLOT,
            &plain,
        ),
        (
            VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
            CARTOGRAPHY_TABLE_MAP_SLOT,
            &plain,
        ),
        (
            VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
            CARTOGRAPHY_TABLE_ADDITIONAL_SLOT,
            &plain,
        ),
    ] {
        assert!(
            !access(menu_type_id, slot_num, stack).may_place,
            "menu {menu_type_id} slot {slot_num}"
        );
    }

    let disabled_crafter_access = local_container_slot_access(
        ctx,
        7,
        Some(VANILLA_MENU_TYPE_CRAFTER_3X3_ID),
        None,
        &[ContainerDataValue { id: 4, value: 1 }],
        4,
        &plain,
        &plain,
    )
    .unwrap();
    assert!(!disabled_crafter_access.may_place);

    for (menu_type_id, result_slot) in [
        (VANILLA_MENU_TYPE_CRAFTING_ID, CRAFTING_MENU_RESULT_SLOT),
        (VANILLA_MENU_TYPE_FURNACE_ID, FURNACE_RESULT_SLOT),
        (VANILLA_MENU_TYPE_GRINDSTONE_ID, GRINDSTONE_RESULT_SLOT),
        (VANILLA_MENU_TYPE_LOOM_ID, LOOM_RESULT_SLOT),
        (VANILLA_MENU_TYPE_MERCHANT_ID, MERCHANT_RESULT_SLOT),
        (VANILLA_MENU_TYPE_SMITHING_ID, SMITHING_RESULT_SLOT),
        (
            VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
            CARTOGRAPHY_TABLE_RESULT_SLOT,
        ),
        (VANILLA_MENU_TYPE_STONECUTTER_ID, STONECUTTER_RESULT_SLOT),
    ] {
        assert!(!access(menu_type_id, result_slot, &plain).may_place);
    }
}

#[test]
fn apply_local_brewing_stand_pickup_all_collects_matching_menu_and_player_stacks() {
    let mut store = WorldStore::new();
    store.set_brewing_potion_item_ids(BTreeSet::from([42]));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BREWING_STAND_ID,
        title: bbb_protocol::RichTextComponent::plain("Brewing Stand"),
    });
    let mut items =
        vec![ProtocolItemStackSummary::empty(); BREWING_STAND_TOTAL_SLOT_COUNT as usize];
    items[BREWING_STAND_BOTTLE_SLOT_START as usize] = item_stack(42, 2);
    items[BREWING_STAND_PLAYER_MAIN_START as usize] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items,
        carried_item: item_stack(42, 1),
    });

    let pickup_all = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BREWING_STAND_PLAYER_MAIN_START + 1,
            button_num: 0,
            input: ProtocolContainerInput::PickupAll,
        })
        .unwrap();

    assert_eq!(
        pickup_all.changed_slots,
        BTreeMap::from([
            (BREWING_STAND_BOTTLE_SLOT_START, ProtocolHashedStack::Empty),
            (BREWING_STAND_PLAYER_MAIN_START, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(pickup_all.carried_item, hashed_item_stack(42, 6));
}

#[test]
fn apply_local_container_quick_craft_left_drag_distributes_evenly() {
    let mut store = WorldStore::new();
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(42, 8),
    });
    assert!(store.open_local_inventory());

    let start = store
        .apply_local_container_click_slot(quick_craft_request(
            -999,
            QUICKCRAFT_HEADER_START,
            QUICKCRAFT_TYPE_CHARITABLE,
        ))
        .unwrap();
    assert!(start.changed_slots.is_empty());
    assert_eq!(start.carried_item, hashed_item_stack(42, 8));

    let add_first = store
        .apply_local_container_click_slot(quick_craft_request(
            INVENTORY_MENU_MAIN_START,
            QUICKCRAFT_HEADER_CONTINUE,
            QUICKCRAFT_TYPE_CHARITABLE,
        ))
        .unwrap();
    assert!(add_first.changed_slots.is_empty());
    assert_eq!(add_first.carried_item, hashed_item_stack(42, 8));

    let add_second = store
        .apply_local_container_click_slot(quick_craft_request(
            INVENTORY_MENU_MAIN_START + 1,
            QUICKCRAFT_HEADER_CONTINUE,
            QUICKCRAFT_TYPE_CHARITABLE,
        ))
        .unwrap();
    assert!(add_second.changed_slots.is_empty());
    assert_eq!(add_second.carried_item, hashed_item_stack(42, 8));

    let finish = store
        .apply_local_container_click_slot(quick_craft_request(
            -999,
            QUICKCRAFT_HEADER_END,
            QUICKCRAFT_TYPE_CHARITABLE,
        ))
        .unwrap();

    assert_eq!(finish.input, ProtocolContainerInput::QuickCraft);
    assert_eq!(
        finish.changed_slots,
        BTreeMap::from([
            (INVENTORY_MENU_MAIN_START, hashed_item_stack(42, 4)),
            (INVENTORY_MENU_MAIN_START + 1, hashed_item_stack(42, 4)),
        ])
    );
    assert_eq!(finish.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
        item_stack(42, 4)
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START + 1),
        item_stack(42, 4)
    );
    assert_eq!(player_slot_item(&store, 9), item_stack(42, 4));
    assert_eq!(player_slot_item(&store, 10), item_stack(42, 4));
    assert_eq!(
        store.inventory().cursor_item,
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn local_crafter_slot_state_change_mutates_data_plays_sound_and_accepts_correction() {
    let mut store = WorldStore::new();
    store.set_local_player_pose(crate::LocalPlayerPoseState::default());
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTER_3X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafter"),
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![ProtocolItemStackSummary::empty(); CRAFTER_TOTAL_SLOT_COUNT as usize],
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let disable = store
        .plan_local_crafter_slot_state_change(0, ProtocolContainerInput::Pickup, 0)
        .unwrap();
    assert_eq!(
        disable,
        LocalCrafterSlotStateChange {
            container_id: 7,
            slot_id: 0,
            enabled: false,
        }
    );
    assert!(store.apply_local_crafter_slot_state_change(disable));
    assert_eq!(store.open_container_data_value(0), Some(1));
    assert_eq!(
        store.take_pending_local_interaction_sound_actions(),
        vec![LocalInteractionSoundAction::CrafterSlotStateChanged { enabled: false }]
    );
    let disabled_sound = store
        .apply_local_interaction_sound_action(
            LocalInteractionSoundAction::CrafterSlotStateChanged { enabled: false },
            &mut LevelEventSoundRandomState::default(),
        )
        .unwrap();
    let LocalInteractionSoundEffect::Positioned(disabled_sound) = disabled_sound else {
        panic!("crafter slot toggle must be a local-player sound");
    };
    assert_eq!(
        disabled_sound.sound.location.as_deref(),
        Some("minecraft:ui.button.click")
    );
    assert_eq!(disabled_sound.source, "player");
    assert_eq!(disabled_sound.volume, 0.4);
    assert_eq!(disabled_sound.pitch, 0.75);

    assert!(store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 7,
        id: 0,
        value: 0,
    }));
    assert_eq!(store.open_container_data_value(0), Some(0));
    assert!(store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 7,
        id: 0,
        value: 1,
    }));
    let enable = store
        .plan_local_crafter_slot_state_change(0, ProtocolContainerInput::Pickup, 0)
        .unwrap();
    assert_eq!(
        enable,
        LocalCrafterSlotStateChange {
            container_id: 7,
            slot_id: 0,
            enabled: true,
        }
    );
    assert!(store.apply_local_crafter_slot_state_change(enable));
    assert_eq!(store.open_container_data_value(0), Some(0));
    let enabled_action = store
        .take_pending_local_interaction_sound_actions()
        .pop()
        .unwrap();
    let enabled_sound = store
        .apply_local_interaction_sound_action(
            enabled_action,
            &mut LevelEventSoundRandomState::default(),
        )
        .unwrap();
    let LocalInteractionSoundEffect::Positioned(enabled_sound) = enabled_sound else {
        panic!("crafter slot enable must be a local-player sound");
    };
    assert_eq!(enabled_sound.pitch, 1.0);

    assert!(store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 7,
        state_id: 13,
        slot: 0,
        item: item_stack(42, 1),
    }));
    assert_eq!(
        store.plan_local_crafter_slot_state_change(0, ProtocolContainerInput::Pickup, 0),
        None
    );
    assert!(store
        .take_pending_local_interaction_sound_actions()
        .is_empty());
}

#[test]
fn inventory_menu_crafting_result_pickup_preserves_server_components_and_consumes_inputs_once() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    let server_result = item_stack_with_int_component_summary(
        90,
        1,
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
        16,
    );
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = server_result.clone();
    items[1] = item_stack(42, 2);
    items[2] = item_stack(43, 1);
    items[4] = item_stack(44, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: INVENTORY_MENU_CONTAINER_ID,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert!(store.open_local_inventory());

    let pickup = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(
        pickup.changed_slots,
        BTreeMap::from([
            (0, ProtocolHashedStack::Empty),
            (1, hashed_item_stack(42, 1)),
            (2, ProtocolHashedStack::Empty),
            (4, hashed_item_stack(44, 2)),
        ])
    );
    assert_eq!(
        pickup.carried_item,
        hashed_int_component_item_stack(90, 1, VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID, 16,)
    );
    assert_eq!(
        inventory_menu_slot_item(&store, 0),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(inventory_menu_slot_item(&store, 1), item_stack(42, 1));
    assert_eq!(
        inventory_menu_slot_item(&store, 2),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(inventory_menu_slot_item(&store, 4), item_stack(44, 2));
    assert_eq!(store.inventory().cursor_item, server_result);
}

#[test]
fn crafting_result_quick_move_stops_after_one_client_take() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 2);
    items[5] = item_stack(43, 2);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::QuickMove
        ))
    );
    let slots = &open_container_slots(&store);
    assert_eq!(slots[0].item, item_stack(90, 1));
    assert_eq!(slots[1].item, item_stack(42, 2));
    assert_eq!(slots[5].item, item_stack(43, 2));
    assert_eq!(slots[45].item, ProtocolItemStackSummary::empty());
}

#[test]
fn crafting_result_server_correction_replaces_default_client_prediction() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 2);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert!(store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 7,
        state_id: 15,
        slot: 0,
        item: item_stack(91, 2),
    }));
    assert!(store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 7,
        state_id: 16,
        slot: 1,
        item: item_stack(42, 2),
    }));

    let slots = &open_container_slots(&store);
    assert_eq!(slots[0].item, item_stack(91, 2));
    assert_eq!(slots[1].item, item_stack(42, 2));
}

#[test]
fn anvil_rename_response_replaces_result_components_and_cost() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: bbb_protocol::RichTextComponent::plain("Anvil"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); ANVIL_TOTAL_SLOT_COUNT as usize];
    items[ANVIL_INPUT_SLOT as usize] = item_stack(42, 1);
    items[ANVIL_RESULT_SLOT as usize] = item_stack(42, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let mut renamed_result = item_stack(42, 1);
    renamed_result.component_patch.added_type_ids =
        vec![VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID];
    renamed_result.component_patch.custom_name = Some("Sharp Pick".to_string());
    assert!(store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 7,
        state_id: 14,
        slot: ANVIL_RESULT_SLOT,
        item: renamed_result.clone(),
    }));
    assert!(store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 7,
        id: ANVIL_COST_DATA_ID,
        value: 3,
    }));

    assert_eq!(
        open_container_slot_item(&store, ANVIL_RESULT_SLOT),
        renamed_result
    );
    assert_eq!(store.open_container_data_value(ANVIL_COST_DATA_ID), Some(3));
    assert_eq!(
        store
            .inventory()
            .open_container
            .as_ref()
            .map(|container| container.state_id),
        Some(14)
    );
}

#[test]
fn loom_selectable_patterns_follow_direct_and_named_session_holder_order() {
    let mut store = WorldStore::new();
    install_loom_banner_pattern_registry(&mut store);

    open_loom_with_pattern_stack(&mut store, ProtocolItemStackSummary::empty());
    assert_eq!(
        store.loom_selectable_banner_pattern_ids(),
        Some(vec![3, 0, 1])
    );

    let mut direct = item_stack(44, 1);
    direct
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID);
    direct.component_patch.provides_banner_pattern_registry_ids = vec![2, 0];
    open_loom_with_pattern_stack(&mut store, direct);
    assert_eq!(store.loom_selectable_banner_pattern_ids(), Some(vec![2, 0]));

    let mut named = item_stack(44, 1);
    named
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID);
    named.component_patch.provides_banner_pattern_tag = Some("example:item_patterns".to_string());
    open_loom_with_pattern_stack(&mut store, named);
    assert_eq!(store.loom_selectable_banner_pattern_ids(), Some(vec![2, 0]));

    let mut unknown = item_stack(44, 1);
    unknown
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID);
    unknown.component_patch.provides_banner_pattern_registry_ids = vec![9];
    open_loom_with_pattern_stack(&mut store, unknown);
    assert_eq!(store.loom_selectable_banner_pattern_ids(), Some(Vec::new()));

    let mut conflicting = item_stack(44, 1);
    conflicting
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID);
    conflicting
        .component_patch
        .provides_banner_pattern_registry_ids = vec![2, 0];
    conflicting.component_patch.provides_banner_pattern_tag =
        Some("example:item_patterns".to_string());
    open_loom_with_pattern_stack(&mut store, conflicting);
    assert_eq!(store.loom_selectable_banner_pattern_ids(), Some(Vec::new()));

    open_loom_with_pattern_stack(&mut store, item_stack(44, 1));
    assert_eq!(store.loom_selectable_banner_pattern_ids(), Some(Vec::new()));
}

#[test]
fn loom_selectable_patterns_honor_effective_defaults_overrides_and_removals() {
    let mut store = WorldStore::new();
    install_loom_banner_pattern_registry(&mut store);
    store.set_default_item_component_codec_values(BTreeMap::from([(
        44,
        BTreeMap::from([(
            VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
            serde_json::json!("#example:item_patterns"),
        )]),
    )]));
    store.rebuild_item_component_defaults().unwrap();

    open_loom_with_pattern_stack(&mut store, item_stack(44, 1));
    assert_eq!(store.loom_selectable_banner_pattern_ids(), Some(vec![2, 0]));

    let mut explicit = item_stack(44, 1);
    explicit
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID);
    explicit
        .component_patch
        .provides_banner_pattern_registry_ids = vec![1];
    open_loom_with_pattern_stack(&mut store, explicit);
    assert_eq!(store.loom_selectable_banner_pattern_ids(), Some(vec![1]));

    let mut removed = item_stack(44, 1);
    removed
        .component_patch
        .removed_type_ids
        .push(VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID);
    open_loom_with_pattern_stack(&mut store, removed);
    assert_eq!(store.loom_selectable_banner_pattern_ids(), Some(Vec::new()));
}

#[test]
fn local_container_button_click_reproduces_loom_result_and_ui_sound() {
    let mut store = WorldStore::new();
    install_loom_banner_pattern_registry(&mut store);
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
        title: bbb_protocol::RichTextComponent::plain("Loom"),
    });
    let mut banner = item_stack(42, 3);
    banner
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID);
    banner.component_patch.banner_pattern_layers.push(
        bbb_protocol::packets::BannerPatternLayerSummary {
            registry_id: Some(1),
            asset_id: None,
            translation_key: None,
            color_id: VANILLA_DYE_COLOR_YELLOW_ID,
        },
    );
    let mut dye = item_stack(43, 2);
    dye.component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_DYE_TYPE_ID);
    dye.component_patch.dye_color_id = Some(VANILLA_DYE_COLOR_RED_ID);
    let mut items = vec![ProtocolItemStackSummary::empty(); LOOM_TOTAL_SLOT_COUNT as usize];
    items[LOOM_BANNER_SLOT as usize] = banner.clone();
    items[LOOM_DYE_SLOT as usize] = dye;
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 7,
        id: 0,
        value: -1,
    });

    assert_eq!(store.apply_local_container_button_click(1), Some(7));
    assert_eq!(store.open_container_data_value(0), Some(1));
    let result = open_container_slot_item(&store, LOOM_RESULT_SLOT);
    assert_eq!(result.item_id, banner.item_id);
    assert_eq!(result.count, 1);
    assert_eq!(
        result.component_patch.banner_pattern_layers,
        vec![
            bbb_protocol::packets::BannerPatternLayerSummary {
                registry_id: Some(1),
                asset_id: None,
                translation_key: None,
                color_id: VANILLA_DYE_COLOR_YELLOW_ID,
            },
            bbb_protocol::packets::BannerPatternLayerSummary {
                registry_id: Some(0),
                asset_id: None,
                translation_key: None,
                color_id: VANILLA_DYE_COLOR_RED_ID,
            },
        ]
    );
    assert_eq!(
        store.take_pending_local_interaction_sound_actions(),
        vec![LocalInteractionSoundAction::LoomSelectPattern]
    );

    let effect = store
        .apply_local_interaction_sound_action(
            LocalInteractionSoundAction::LoomSelectPattern,
            &mut LevelEventSoundRandomState::default(),
        )
        .unwrap();
    let LocalInteractionSoundEffect::Local(sound) = effect else {
        panic!("loom selection must be a non-attenuated UI sound");
    };
    assert_eq!(
        sound.sound.location.as_deref(),
        Some("minecraft:ui.loom.select_pattern")
    );
    assert_eq!(sound.source, "ui");
    assert_eq!(sound.volume, 0.25);
    assert_eq!(sound.pitch, 1.0);
}

#[test]
fn local_container_button_click_matches_stonecutter_client_recipe_projection() {
    let mut store = WorldStore::new();
    store.apply_update_recipes(update_stonecutter_recipes(vec![
        stonecutter_recipe(vec![42]),
        stonecutter_recipe(vec![42, 43]),
    ]));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 8,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); STONECUTTER_TOTAL_SLOT_COUNT as usize];
    items[STONECUTTER_INPUT_SLOT as usize] = item_stack(42, 4);
    items[STONECUTTER_RESULT_SLOT as usize] = item_stack(99, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 8,
        state_id: 14,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 8,
        id: 0,
        value: -1,
    });

    assert_eq!(store.stonecutter_visible_recipe_count(), Some(2));
    assert_eq!(store.apply_local_container_button_click(1), Some(8));
    assert_eq!(store.open_container_data_value(0), Some(1));
    assert_eq!(
        open_container_slot_item(&store, STONECUTTER_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(store.apply_local_container_button_click(1), None);
    assert_eq!(
        store.take_pending_local_interaction_sound_actions(),
        vec![LocalInteractionSoundAction::StonecutterSelectRecipe]
    );

    // Vanilla returns true for a non-selected id outside recipesForInput,
    // leaving selection/result untouched while still playing and sending.
    assert_eq!(store.apply_local_container_button_click(11), Some(8));
    assert_eq!(store.open_container_data_value(0), Some(1));
    assert_eq!(
        store.take_pending_local_interaction_sound_actions(),
        vec![LocalInteractionSoundAction::StonecutterSelectRecipe]
    );
}

#[test]
fn local_container_button_click_enforces_enchantment_client_eligibility_only() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 9,
        menu_type_id: VANILLA_MENU_TYPE_ENCHANTMENT_ID,
        title: bbb_protocol::RichTextComponent::plain("Enchant"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); ENCHANTMENT_TOTAL_SLOT_COUNT as usize];
    items[ENCHANTMENT_INPUT_SLOT as usize] = item_stack(42, 1);
    items[ENCHANTMENT_LAPIS_SLOT as usize] = item_stack(43, 2);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 9,
        state_id: 15,
        items: items.clone(),
        carried_item: ProtocolItemStackSummary::empty(),
    });
    store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 9,
        id: 1,
        value: 12,
    });
    store.apply_player_experience(ProtocolPlayerExperience {
        progress: 0.5,
        level: 30,
        total: 200,
    });

    assert_eq!(store.apply_local_container_button_click(1), Some(9));
    assert_eq!(
        open_container_slot_item(&store, ENCHANTMENT_INPUT_SLOT),
        items[ENCHANTMENT_INPUT_SLOT as usize]
    );
    assert_eq!(
        open_container_slot_item(&store, ENCHANTMENT_LAPIS_SLOT),
        items[ENCHANTMENT_LAPIS_SLOT as usize]
    );
    assert_eq!(store.local_player().experience.unwrap().level, 30);
    assert_eq!(store.apply_local_container_button_click(2), None);

    apply_player_instabuild(&mut store, true);
    store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 9,
        id: 2,
        value: 18,
    });
    assert_eq!(store.apply_local_container_button_click(2), Some(9));
}

#[test]
fn local_container_button_click_keeps_lectern_state_server_owned() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 10,
        menu_type_id: VANILLA_MENU_TYPE_LECTERN_ID,
        title: bbb_protocol::RichTextComponent::plain("Lectern"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 1];
    items[0] = item_stack(42, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 10,
        state_id: 16,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 10,
        id: 0,
        value: 4,
    });

    assert_eq!(store.apply_local_container_button_click(2), Some(10));
    assert_eq!(store.open_container_data_value(0), Some(4));
    assert_eq!(store.apply_local_container_button_click(3), Some(10));
    assert_eq!(
        open_container_slot_item(&store, 0),
        item_stack(42, 1),
        "LecternScreen does not invoke its local menu before sending"
    );
}

#[test]
fn set_local_merchant_selected_offer_autofills_payment_slots_from_offer_costs() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); MERCHANT_TOTAL_SLOT_COUNT as usize];
    items[MERCHANT_PAYMENT_SLOT_1 as usize] = item_stack(99, 2);
    items[MERCHANT_PAYMENT_SLOT_2 as usize] = item_stack(98, 1);
    items[MERCHANT_PLAYER_MAIN_START as usize] = item_stack(42, 5);
    items[(MERCHANT_PLAYER_MAIN_START + 1) as usize] = item_stack(43, 6);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert!(store.apply_merchant_offers(ProtocolMerchantOffers {
        container_id: 7,
        offers: vec![ProtocolMerchantOffer {
            buy_a: item_cost(42, 3),
            sell: item_stack(90, 1),
            buy_b: Some(item_cost(43, 2)),
            is_out_of_stock: false,
            uses: 0,
            max_uses: 12,
            xp: 8,
            special_price_diff: 0,
            price_multiplier: 0.05,
            demand: 0,
        }],
        villager_level: 3,
        villager_xp: 120,
        show_progress: true,
        can_restock: false,
    }));

    assert!(store.set_local_merchant_selected_offer(0));

    let offers = store
        .inventory()
        .open_container
        .as_ref()
        .and_then(|container| container.merchant_offers.as_ref())
        .unwrap();
    assert_eq!(offers.local_selected_offer_index, 0);
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_1),
        item_stack(42, 5)
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_2),
        item_stack(43, 6)
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PLAYER_MAIN_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PLAYER_MAIN_START + 1),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_HOTBAR_END - 1),
        item_stack(99, 2)
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_HOTBAR_END - 2),
        item_stack(98, 1)
    );
}

#[test]
fn set_local_merchant_selected_offer_autofills_matching_component_predicate_cost_only() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    let mut mismatched_payment = item_stack(42, 4);
    mismatched_payment.component_patch.added_type_ids =
        vec![VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID];
    mismatched_payment.component_patch.item_model = Some("minecraft:diamond".to_string());

    let plain_payment = item_stack(42, 2);

    let mut matching_payment = item_stack(42, 5);
    matching_payment.component_patch.added_type_ids =
        vec![VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID];
    matching_payment.component_patch.item_model = Some("minecraft:emerald".to_string());

    let mut items = vec![ProtocolItemStackSummary::empty(); MERCHANT_TOTAL_SLOT_COUNT as usize];
    items[MERCHANT_PLAYER_MAIN_START as usize] = mismatched_payment.clone();
    items[(MERCHANT_PLAYER_MAIN_START + 1) as usize] = plain_payment.clone();
    items[(MERCHANT_PLAYER_MAIN_START + 2) as usize] = matching_payment.clone();
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 15,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    let mut cost = item_cost(42, 1);
    cost.component_predicate.components.added_type_ids =
        vec![VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID];
    cost.component_predicate.components.item_model = Some("minecraft:emerald".to_string());
    assert!(store.apply_merchant_offers(ProtocolMerchantOffers {
        container_id: 7,
        offers: vec![ProtocolMerchantOffer {
            buy_a: cost,
            sell: item_stack(90, 1),
            buy_b: None,
            is_out_of_stock: false,
            uses: 0,
            max_uses: 12,
            xp: 8,
            special_price_diff: 0,
            price_multiplier: 0.05,
            demand: 0,
        }],
        villager_level: 3,
        villager_xp: 120,
        show_progress: true,
        can_restock: false,
    }));

    assert!(store.set_local_merchant_selected_offer(0));

    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_1),
        matching_payment
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PLAYER_MAIN_START),
        mismatched_payment
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PLAYER_MAIN_START + 1),
        plain_payment
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PLAYER_MAIN_START + 2),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn set_local_merchant_selected_offer_matches_prototype_default_and_skips_removed_component() {
    let mut store = WorldStore::new();
    let default_patch = ProtocolDataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID],
        item_model: Some("minecraft:emerald".to_string()),
        ..Default::default()
    };
    store
        .items
        .commit_item_component_defaults(BTreeMap::from([(42, default_patch)]));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    let mut removed_component_stack = item_stack(42, 3);
    removed_component_stack.component_patch.removed_type_ids =
        vec![VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID];
    let default_component_stack = item_stack(42, 4);

    let mut items = vec![ProtocolItemStackSummary::empty(); MERCHANT_TOTAL_SLOT_COUNT as usize];
    items[MERCHANT_PLAYER_MAIN_START as usize] = removed_component_stack.clone();
    items[(MERCHANT_PLAYER_MAIN_START + 1) as usize] = default_component_stack.clone();
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 15,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    let mut cost = item_cost(42, 1);
    cost.component_predicate.components.added_type_ids =
        vec![VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID];
    cost.component_predicate.components.item_model = Some("minecraft:emerald".to_string());
    assert!(store.apply_merchant_offers(ProtocolMerchantOffers {
        container_id: 7,
        offers: vec![ProtocolMerchantOffer {
            buy_a: cost,
            sell: item_stack(90, 1),
            buy_b: None,
            is_out_of_stock: false,
            uses: 0,
            max_uses: 12,
            xp: 8,
            special_price_diff: 0,
            price_multiplier: 0.05,
            demand: 0,
        }],
        villager_level: 3,
        villager_xp: 120,
        show_progress: true,
        can_restock: false,
    }));

    assert!(store.set_local_merchant_selected_offer(0));
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_1),
        default_component_stack
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PLAYER_MAIN_START),
        removed_component_stack
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PLAYER_MAIN_START + 1),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_cartography_table_player_map_id_quick_move_routes_to_map_slot() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
        title: bbb_protocol::RichTextComponent::plain("Cartography Table"),
    });
    let mut items =
        vec![ProtocolItemStackSummary::empty(); CARTOGRAPHY_TABLE_TOTAL_SLOT_COUNT as usize];
    items[CARTOGRAPHY_TABLE_PLAYER_MAIN_START as usize] = map_id_item_stack(42, 1, 7);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let map_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: CARTOGRAPHY_TABLE_PLAYER_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        map_move.changed_slots,
        BTreeMap::from([
            (
                CARTOGRAPHY_TABLE_MAP_SLOT,
                hashed_map_id_item_stack(42, 1, 7)
            ),
            (
                CARTOGRAPHY_TABLE_PLAYER_MAIN_START,
                ProtocolHashedStack::Empty
            ),
        ])
    );
    assert_eq!(map_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_MAP_SLOT),
        map_id_item_stack(42, 1, 7)
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_PLAYER_MAIN_START),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_stonecutter_quick_move_routes_valid_recipe_input_to_input_slot() {
    let mut store = WorldStore::new();
    store.apply_update_recipes(update_stonecutter_recipes(vec![stonecutter_recipe(vec![
        42, 43,
    ])]));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); STONECUTTER_TOTAL_SLOT_COUNT as usize];
    items[STONECUTTER_PLAYER_MAIN_START as usize] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: STONECUTTER_PLAYER_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (STONECUTTER_INPUT_SLOT, hashed_item_stack(42, 3)),
            (STONECUTTER_PLAYER_MAIN_START, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, STONECUTTER_INPUT_SLOT),
        item_stack(42, 3)
    );
    assert_eq!(
        open_container_slot_item(&store, STONECUTTER_PLAYER_MAIN_START),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn every_generated_menu_type_has_known_container_click_rules() {
    assert_eq!(VANILLA_MENU_TYPES.len(), 25);
    for menu_type in VANILLA_MENU_TYPES {
        assert!(
            menu_type_has_known_slot_rules(Some(menu_type.id)),
            "generated menu {} ({}) has no container-click rules",
            menu_type.constant_name,
            menu_type.resource_id
        );
    }
}

#[test]
fn every_generated_menu_type_uses_the_same_open_replace_close_lifecycle() {
    let mut store = WorldStore::new();
    let mut expected_revision = 0;

    for menu_type in VANILLA_MENU_TYPES {
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: menu_type.id + 1,
            menu_type_id: menu_type.id,
            title: bbb_protocol::RichTextComponent::plain(menu_type.resource_id),
        });
        expected_revision += 1;

        let container = store.inventory().open_container.as_ref().unwrap();
        assert_eq!(container.container_id, menu_type.id + 1);
        assert_eq!(container.menu_type_id, Some(menu_type.id));
        assert_eq!(
            container.title.as_ref().unwrap().text(),
            menu_type.resource_id
        );
        assert_eq!(store.inventory().open_container_revision, expected_revision);

        assert!(store.apply_container_close(ProtocolContainerClose {
            container_id: menu_type.id + 1,
        }));
        expected_revision += 1;
        assert!(store.inventory().open_container.is_none());
        assert_eq!(store.inventory().open_container_revision, expected_revision);
    }
}

#[test]
fn open_screen_exact_title_is_stored_and_survives_set_content() {
    let title = bbb_protocol::RichTextComponent::try_from_component_nbt(
        bbb_protocol::NbtSummaryValue::Compound(vec![
            bbb_protocol::NbtSummaryEntry {
                name: "text".into(),
                value: bbb_protocol::NbtSummaryValue::string("Chest"),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "bold".into(),
                value: bbb_protocol::NbtSummaryValue::Byte(1),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "color".into(),
                value: bbb_protocol::NbtSummaryValue::string("#ff5555"),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "click_event".into(),
                value: bbb_protocol::NbtSummaryValue::Compound(vec![
                    bbb_protocol::NbtSummaryEntry {
                        name: "action".into(),
                        value: bbb_protocol::NbtSummaryValue::string("run_command"),
                    },
                    bbb_protocol::NbtSummaryEntry {
                        name: "command".into(),
                        value: bbb_protocol::NbtSummaryValue::string("/menu"),
                    },
                ]),
            },
        ]),
    )
    .unwrap();
    let exact = title.component().clone();
    let styled = title.styled().to_vec();
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 5,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X1_ID,
        title,
    });
    let stored = store
        .inventory()
        .open_container
        .as_ref()
        .unwrap()
        .title
        .as_ref()
        .unwrap();
    assert_eq!(stored.component(), &exact);
    assert_eq!(stored.styled(), styled);
    assert_eq!(
        stored.styled()[0].style.click_event,
        Some(bbb_protocol::ComponentClickEvent::RunCommand {
            command: "/menu".into(),
        })
    );

    // Content refresh keeps the exact trusted component and its projections.
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 5,
        state_id: 1,
        items: vec![ProtocolItemStackSummary::empty(); 9],
        carried_item: ProtocolItemStackSummary::empty(),
    });
    let container = store.inventory().open_container.as_ref().unwrap();
    let stored = container.title.as_ref().unwrap();
    assert_eq!(stored.text(), "Chest");
    assert_eq!(stored.component(), &exact);
    assert_eq!(stored.styled(), styled);
}
