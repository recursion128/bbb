use super::*;
use crate::entities::{
    vanilla_entity_metadata_baby_accessor_for_entity_type, VANILLA_ENTITY_TYPE_DONKEY_ID,
    VANILLA_ENTITY_TYPE_HORSE_ID, VANILLA_ENTITY_TYPE_LLAMA_ID, VANILLA_ENTITY_TYPE_NAUTILUS_ID,
};
use crate::{LevelEventSoundRandomState, LocalInteractionSoundEffect};
use bbb_pack::RegistryDependentItemComponentInitializer;
use bbb_protocol::builtin_enum_component_values::{
    VANILLA_DYE_COLOR_RED_ID, VANILLA_DYE_COLOR_YELLOW_ID,
};
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID, VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
    VANILLA_DATA_COMPONENT_DYE_TYPE_ID, VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID,
    VANILLA_DATA_COMPONENT_LOCK_TYPE_ID, VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
    VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
    VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
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
            added: 2,
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
        },
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
    assert_eq!(store.equipment_item(123, EquipmentSlot::Head), Some(stack));
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
fn container_player_section_updates_mirror_into_the_backpack() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    // A 9x3 chest menu has 27 container slots, then main 27..54 mapping to
    // backpack 9-35 and hotbar 54..63 mapping to backpack 0-8. Vanilla's
    // menus share the player inventory object, so a server refresh of the
    // trailing section is a backpack update.
    let mut items = vec![ProtocolItemStackSummary::empty(); 63];
    items[27] = item_stack(42, 3);
    items[62] = item_stack(43, 5);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 2,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let backpack = |store: &WorldStore, slot: i32| {
        store
            .inventory()
            .player_slots
            .iter()
            .find(|row| row.slot == slot)
            .map(|row| row.item.clone())
    };
    assert_eq!(backpack(&store, 9), Some(item_stack(42, 3)));
    assert_eq!(backpack(&store, 8), Some(item_stack(43, 5)));

    // A single-slot refresh in the player section mirrors the same way.
    store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 7,
        state_id: 3,
        slot: 27,
        item: ProtocolItemStackSummary::empty(),
    });
    assert_eq!(backpack(&store, 9), Some(ProtocolItemStackSummary::empty()));

    // Closing the chest and opening the survival inventory must show the
    // mirrored rows in the menu projection.
    store.apply_container_close(ProtocolContainerClose { container_id: 7 });
    assert!(store.open_local_inventory());
    let menu_hotbar_last = store
        .inventory()
        .inventory_menu
        .slots
        .iter()
        .find(|row| row.slot == 44)
        .map(|row| row.item.clone());
    assert_eq!(menu_hotbar_last, Some(item_stack(43, 5)));
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
fn mount_equipment_visibility_follows_vanilla_horse_type_tags() {
    assert_eq!(
        mount_visibility_for_entity(VANILLA_ENTITY_TYPE_HORSE_ID, Vec::new()),
        Some(MountEquipmentSlotVisibility {
            saddle: true,
            body: Some(MountArmorSlotKind::Horse),
        })
    );
    assert_eq!(
        mount_visibility_for_entity(VANILLA_ENTITY_TYPE_LLAMA_ID, Vec::new()),
        Some(MountEquipmentSlotVisibility {
            saddle: false,
            body: Some(MountArmorSlotKind::Llama),
        })
    );
    assert_eq!(
        mount_visibility_for_entity(VANILLA_ENTITY_TYPE_DONKEY_ID, Vec::new()),
        Some(MountEquipmentSlotVisibility {
            saddle: false,
            body: None,
        })
    );
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
fn apply_local_mount_quick_move_routes_non_equipment_player_item_to_mount_inventory() {
    let mut store = WorldStore::new();
    store.set_default_item_equipment_slots(BTreeMap::from([(90, ItemEquipmentSlot::Saddle)]));
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
    items[17] = item_stack(43, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 17,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (MOUNT_INVENTORY_START, hashed_item_stack(43, 3)),
            (17, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, MOUNT_INVENTORY_START),
        item_stack(43, 3)
    );
    assert_eq!(
        open_container_slot_item(&store, 17),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_mount_quick_move_routes_saddle_to_equipment_slot() {
    let mut store = WorldStore::new();
    store.set_default_item_equipment_slots(BTreeMap::from([(90, ItemEquipmentSlot::Saddle)]));
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
    items[17] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 17,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (0, hashed_item_stack(90, 1)),
            (17, ProtocolHashedStack::Empty)
        ])
    );
    assert_eq!(open_container_slot_item(&store, 0), item_stack(90, 1));
    assert_eq!(
        open_container_slot_item(&store, 17),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_mount_quick_move_routes_matching_body_armor_to_equipment_slot() {
    let mut store = WorldStore::new();
    store.set_default_item_equipment_slots(BTreeMap::from([
        (91, ItemEquipmentSlot::Body),
        (92, ItemEquipmentSlot::Body),
    ]));
    store.set_default_mount_body_armor_kinds(BTreeMap::from([
        (91, MountArmorSlotKind::Horse),
        (92, MountArmorSlotKind::Llama),
    ]));
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
    items[17] = item_stack(91, 1);
    items[18] = item_stack(92, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 15,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let horse_armor = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 17,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        horse_armor.changed_slots,
        BTreeMap::from([
            (1, hashed_item_stack(91, 1)),
            (17, ProtocolHashedStack::Empty)
        ])
    );

    let llama_armor = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 18,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        llama_armor.changed_slots,
        BTreeMap::from([
            (MOUNT_INVENTORY_START, hashed_item_stack(92, 1)),
            (18, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(open_container_slot_item(&store, 1), item_stack(91, 1));
    assert_eq!(
        open_container_slot_item(&store, MOUNT_INVENTORY_START),
        item_stack(92, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, 17),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, 18),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_mount_quick_move_keeps_component_patched_player_item_server_authoritative() {
    let mut store = WorldStore::new();
    store.set_default_item_equipment_slots(BTreeMap::from([(90, ItemEquipmentSlot::Saddle)]));
    store.apply_add_entity(protocol_add_entity_with_type(
        42,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    store.apply_mount_screen_open(ProtocolMountScreenOpen {
        container_id: 7,
        inventory_columns: 5,
        entity_id: 42,
    });
    let mut patched = item_stack(90, 1);
    patched.component_patch.added = 1;
    patched.component_patch.added_type_ids = vec![1];
    let mut items = vec![ProtocolItemStackSummary::empty(); 53];
    items[17] = patched.clone();
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 16,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let err = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 17,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap_err();

    assert_eq!(
        err,
        ContainerClickBuildError::UnsupportedLocalClickInput(ProtocolContainerInput::QuickMove)
    );
    assert_eq!(open_container_slot_item(&store, 17), patched);
}

#[test]
fn apply_local_mount_quick_move_keeps_unknown_body_armor_kind_server_authoritative() {
    let mut store = WorldStore::new();
    store.set_default_item_equipment_slots(BTreeMap::from([(93, ItemEquipmentSlot::Body)]));
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
    items[17] = item_stack(93, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 17,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let err = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 17,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap_err();

    assert_eq!(
        err,
        ContainerClickBuildError::UnsupportedLocalClickInput(ProtocolContainerInput::QuickMove)
    );
    assert_eq!(open_container_slot_item(&store, 17), item_stack(93, 1));
}

#[test]
fn apply_local_mount_pickup_honors_equipment_and_storage_slot_rules() {
    let mut store = WorldStore::new();
    store.set_default_item_component_type_ids(BTreeMap::from([
        (
            90,
            BTreeSet::from([VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID]),
        ),
        (
            91,
            BTreeSet::from([VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID]),
        ),
        (
            92,
            BTreeSet::from([VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID]),
        ),
    ]));
    store.set_default_item_equipment_slots(BTreeMap::from([
        (90, ItemEquipmentSlot::Saddle),
        (91, ItemEquipmentSlot::Body),
        (92, ItemEquipmentSlot::Body),
    ]));
    store.set_default_mount_body_armor_kinds(BTreeMap::from([
        (91, MountArmorSlotKind::Horse),
        (92, MountArmorSlotKind::Llama),
    ]));
    store.apply_add_entity(protocol_add_entity_with_type(
        42,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    store.apply_mount_screen_open(ProtocolMountScreenOpen {
        container_id: 7,
        inventory_columns: 5,
        entity_id: 42,
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 18,
        items: vec![ProtocolItemStackSummary::empty(); 53],
        carried_item: item_stack(90, 3),
    });

    let saddle = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MOUNT_SADDLE_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(
        saddle.changed_slots,
        BTreeMap::from([(MOUNT_SADDLE_SLOT, hashed_item_stack(90, 1))])
    );
    assert_eq!(saddle.carried_item, hashed_item_stack(90, 2));

    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(92, 1),
    });
    let mismatched_body = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MOUNT_BODY_ARMOR_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert!(mismatched_body.changed_slots.is_empty());
    assert_eq!(mismatched_body.carried_item, hashed_item_stack(92, 1));

    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(91, 1),
    });
    let matching_body = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MOUNT_BODY_ARMOR_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(
        matching_body.changed_slots,
        BTreeMap::from([(MOUNT_BODY_ARMOR_SLOT, hashed_item_stack(91, 1))])
    );

    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(43, 3),
    });
    let storage = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MOUNT_INVENTORY_START,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(
        storage.changed_slots,
        BTreeMap::from([(MOUNT_INVENTORY_START, hashed_item_stack(43, 3))])
    );
    assert_eq!(storage.carried_item, ProtocolHashedStack::Empty);
}

#[test]
fn apply_local_mount_equipment_pickup_falls_back_for_enchanted_stack() {
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
    let mut enchanted = item_stack(90, 1);
    enchanted.component_patch.added = 1;
    enchanted.component_patch.added_type_ids =
        vec![bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID];
    enchanted.component_patch.enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
        holder_id: 0,
        level: 1,
    }];
    let mut items = vec![ProtocolItemStackSummary::empty(); 53];
    items[MOUNT_SADDLE_SLOT as usize] = enchanted.clone();
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 19,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MOUNT_SADDLE_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::Pickup
        ))
    );
    assert_eq!(
        open_container_slot_item(&store, MOUNT_SADDLE_SLOT),
        enchanted
    );
}

#[test]
fn hotbar_items_merge_player_inventory_and_inventory_menu_slots() {
    let mut store = WorldStore::new();

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(10, 1),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 8,
        item: item_stack(18, 1),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 9,
        item: item_stack(99, 1),
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 0,
        state_id: 1,
        items: (0..45)
            .map(|slot| {
                if slot == 36 {
                    item_stack(20, 2)
                } else if slot == 44 {
                    item_stack(28, 2)
                } else {
                    ProtocolItemStackSummary::empty()
                }
            })
            .collect(),
        carried_item: ProtocolItemStackSummary::empty(),
    });
    store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 0,
        state_id: 2,
        slot: 40,
        item: item_stack(24, 3),
    });

    let hotbar = store.inventory().hotbar_items();
    assert_eq!(hotbar[0], item_stack(20, 2));
    assert_eq!(hotbar[4], item_stack(24, 3));
    assert_eq!(hotbar[8], item_stack(28, 2));
    assert_eq!(hotbar[1], ProtocolItemStackSummary::empty());
}

#[test]
fn container_zero_content_updates_inventory_menu_without_opening_local_screen() {
    let mut store = WorldStore::new();

    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: INVENTORY_MENU_CONTAINER_ID,
        state_id: 7,
        items: vec![ProtocolItemStackSummary::empty(), item_stack(42, 3)],
        carried_item: item_stack(99, 1),
    });

    assert!(!store.local_inventory_is_open());
    assert_eq!(store.open_container_id(), None);
    assert!(store.inventory().open_container.is_none());
    assert_eq!(store.inventory().inventory_menu.container_id, 0);
    assert_eq!(store.inventory().inventory_menu.state_id, 7);
    assert_eq!(
        store.inventory().inventory_menu.slots[1].item,
        item_stack(42, 3)
    );
    assert_eq!(store.inventory().cursor_item, item_stack(99, 1));
}

#[test]
fn open_local_inventory_builds_container_zero_view_from_player_inventory() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(10, 1),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_CHEST_SLOT,
        item: item_stack(VANILLA_ITEM_ELYTRA_ID, 1),
    });
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(99, 1),
    });

    assert!(store.open_local_inventory());
    assert!(!store.open_local_inventory());
    assert!(store.local_inventory_is_open());
    assert_eq!(store.open_container_id(), Some(INVENTORY_MENU_CONTAINER_ID));

    let inventory_menu = &store.inventory().inventory_menu;
    assert_eq!(inventory_menu.container_id, INVENTORY_MENU_CONTAINER_ID);
    assert_eq!(inventory_menu.slots.len(), 46);
    assert_eq!(
        inventory_menu
            .slots
            .iter()
            .find(|slot| slot.slot == INVENTORY_MENU_HOTBAR_START)
            .unwrap()
            .item,
        item_stack(10, 1)
    );
    assert_eq!(
        inventory_menu
            .slots
            .iter()
            .find(|slot| slot.slot == 6)
            .unwrap()
            .item,
        item_stack(VANILLA_ITEM_ELYTRA_ID, 1)
    );

    let click = store
        .build_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(click.container_id, INVENTORY_MENU_CONTAINER_ID);
    assert_eq!(click.slot_num, INVENTORY_MENU_HOTBAR_START);
    assert_eq!(
        click.carried_item,
        ProtocolHashedStack::Item(ProtocolHashedItemStack {
            item_id: 99,
            count: 1,
            components: ProtocolHashedComponentPatch::default(),
        })
    );
}

#[test]
fn local_item_use_prefers_offhand_only_when_selected_hotbar_slot_is_empty() {
    let mut store = WorldStore::new();

    assert!(!store.local_item_use_prefers_offhand());

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: item_stack(99, 1),
    });
    assert!(store.local_item_use_prefers_offhand());

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(10, 1),
    });
    assert!(!store.local_item_use_prefers_offhand());

    assert!(store.set_local_selected_hotbar_slot(1));
    assert!(store.local_item_use_prefers_offhand());

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: ProtocolItemStackSummary::empty(),
    });
    assert!(!store.local_item_use_prefers_offhand());
}

#[test]
fn local_item_use_reads_inventory_menu_offhand_slot_when_player_slot_is_absent() {
    let mut store = WorldStore::new();

    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: INVENTORY_MENU_CONTAINER_ID,
        state_id: 1,
        items: (0..46)
            .map(|slot| {
                if slot == INVENTORY_MENU_OFFHAND_SLOT {
                    item_stack(99, 1)
                } else {
                    ProtocolItemStackSummary::empty()
                }
            })
            .collect(),
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert!(store.local_item_use_prefers_offhand());

    store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: INVENTORY_MENU_CONTAINER_ID,
        state_id: 2,
        slot: INVENTORY_MENU_OFFHAND_SLOT,
        item: ProtocolItemStackSummary::empty(),
    });
    assert!(!store.local_item_use_prefers_offhand());
}

#[test]
fn local_selected_main_hand_has_piercing_weapon_true_for_default_item() {
    let mut store = WorldStore::new();
    store.set_default_piercing_weapon_item_ids(BTreeSet::from([-1, 42]));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 1),
    });

    assert!(store.local_selected_main_hand_has_piercing_weapon());
}

#[test]
fn local_selected_main_hand_has_piercing_weapon_true_for_added_component() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack_with_component_summary(
            99,
            1,
            VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
        ),
    });

    assert!(store.local_selected_main_hand_has_piercing_weapon());
}

#[test]
fn local_selected_main_hand_has_piercing_weapon_removed_component_overrides_default_and_added() {
    let mut store = WorldStore::new();
    store.set_default_piercing_weapon_item_ids(BTreeSet::from([42]));
    let mut item =
        item_stack_with_component_summary(42, 1, VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID);
    item.component_patch.removed_type_ids = vec![VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID];
    store.apply_set_player_inventory(ProtocolSetPlayerInventory { slot: 0, item });

    assert!(!store.local_selected_main_hand_has_piercing_weapon());
}

#[test]
fn local_selected_main_hand_has_piercing_weapon_false_for_empty_or_invalid_item() {
    let mut store = WorldStore::new();
    store.set_default_piercing_weapon_item_ids(BTreeSet::from([-1, 42]));

    assert!(!store.local_selected_main_hand_has_piercing_weapon());

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 0),
    });
    assert!(!store.local_selected_main_hand_has_piercing_weapon());

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(-1, 1),
    });
    assert!(!store.local_selected_main_hand_has_piercing_weapon());
}

#[test]
fn local_selected_main_hand_has_piercing_weapon_respects_selected_slot() {
    let mut store = WorldStore::new();
    store.set_default_piercing_weapon_item_ids(BTreeSet::from([42]));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 1),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 1,
        item: item_stack(99, 1),
    });

    assert!(store.local_selected_main_hand_has_piercing_weapon());
    assert!(store.set_local_selected_hotbar_slot(1));
    assert!(!store.local_selected_main_hand_has_piercing_weapon());
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
    store
        .set_default_item_attack_ranges(BTreeMap::from([(-1, default_range), (42, default_range)]));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 1),
    });

    assert_eq!(
        store.local_selected_main_hand_attack_range(),
        Some(default_range)
    );

    let mut patched = item_stack(99, 1);
    patched.component_patch.added = 1;
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
fn local_active_item_attack_range_follows_use_hand_and_spectator_policy() {
    let mut store = WorldStore::new();
    let main_range = item_attack_range(0.0, 3.5);
    let offhand_range = item_attack_range(0.0, 4.5);
    store.set_default_item_attack_ranges(BTreeMap::from([(42, main_range), (99, offhand_range)]));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 1),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: item_stack(99, 1),
    });

    assert_eq!(store.local_active_item_attack_range(), Some(main_range));
    store.set_local_using_item_with_hand(true, InteractionHand::OffHand);
    assert_eq!(store.local_active_item_attack_range(), Some(offhand_range));
    store.take_local_using_item();
    assert_eq!(store.local_active_item_attack_range(), Some(main_range));

    store.apply_game_event(bbb_protocol::packets::GameEvent {
        event_id: 3,
        param: 3.0,
    });
    assert_eq!(store.local_active_item_attack_range(), None);
}

#[test]
fn effective_food_properties_apply_default_patch_and_removal_precedence() {
    let mut store = WorldStore::new();
    store.set_default_item_food_properties(BTreeMap::from([
        (
            -1,
            ItemFoodProperties {
                nutrition: 99,
                saturation_bits: 99.0_f32.to_bits(),
                can_always_eat: true,
            },
        ),
        (
            42,
            ItemFoodProperties {
                nutrition: 4,
                saturation_bits: 2.4_f32.to_bits(),
                can_always_eat: false,
            },
        ),
    ]));

    let default_stack = item_stack(42, 1);
    let effective_default = store.effective_item_stack(&default_stack);
    assert_eq!(effective_default.component_patch.food_nutrition, Some(4));
    assert_eq!(
        effective_default.component_patch.food_saturation_bits,
        Some(2.4_f32.to_bits())
    );
    assert!(!effective_default.component_patch.food_can_always_eat);

    let mut patched = item_stack(42, 1);
    patched.component_patch.added = 1;
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
    store.set_default_item_component_type_ids(BTreeMap::from([(
        42,
        BTreeSet::from([
            VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
            VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
        ]),
    )]));
    store.set_default_item_food_properties(BTreeMap::from([(
        42,
        ItemFoodProperties {
            nutrition: 4,
            saturation_bits: 2.4_f32.to_bits(),
            can_always_eat: false,
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
    always_edible.component_patch.added = 1;
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
    store.set_default_item_attack_ranges(BTreeMap::from([(42, item_attack_range(2.0, 4.5))]));
    let mut item = item_stack(42, 1);
    item.component_patch.added = 1;
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
fn local_selected_main_hand_attack_range_respects_empty_invalid_and_selected_slot() {
    let mut store = WorldStore::new();
    store.set_default_item_attack_ranges(BTreeMap::from([
        (-1, item_attack_range(2.0, 4.5)),
        (42, item_attack_range(2.0, 4.5)),
    ]));

    assert_eq!(store.local_selected_main_hand_attack_range(), None);

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 0),
    });
    assert_eq!(store.local_selected_main_hand_attack_range(), None);

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(-1, 1),
    });
    assert_eq!(store.local_selected_main_hand_attack_range(), None);

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 1),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 1,
        item: item_stack(99, 1),
    });
    assert_eq!(
        store.local_selected_main_hand_attack_range(),
        Some(item_attack_range(2.0, 4.5))
    );
    assert!(store.set_local_selected_hotbar_slot(1));
    assert_eq!(store.local_selected_main_hand_attack_range(), None);
}

#[test]
fn local_attack_minimum_charge_applies_default_patch_and_removal_precedence() {
    let mut store = WorldStore::new();
    store.set_default_item_minimum_attack_charges(BTreeMap::from([
        (-1, 0.75),
        (42, 1.0),
        (43, f32::NAN),
    ]));
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
    patched.component_patch.added = 1;
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
fn local_attack_item_enabled_gate_accepts_empty_and_known_vanilla_stacks_only() {
    let mut store = WorldStore::new();
    assert!(store.local_selected_main_hand_is_enabled());

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 1),
    });
    assert!(store.local_selected_main_hand_is_enabled());

    store.apply_update_enabled_features(bbb_protocol::packets::UpdateEnabledFeatures {
        features: vec!["minecraft:trade_rebalance".to_string()],
    });
    assert!(!store.local_selected_main_hand_is_enabled());

    store.apply_update_enabled_features(bbb_protocol::packets::UpdateEnabledFeatures {
        features: vec!["minecraft:vanilla".to_string()],
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(99_999, 1),
    });
    assert!(!store.local_selected_main_hand_is_enabled());
}

#[test]
fn local_using_item_use_effects_reads_default_and_patch_components() {
    let mut store = WorldStore::new();
    let default_effects = ItemUseEffects {
        can_sprint: true,
        interact_vibrations: false,
        speed_multiplier: 1.0,
    };
    store.set_default_item_use_effects(BTreeMap::from([
        (-1, default_effects),
        (42, default_effects),
    ]));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 1),
    });
    store.set_local_using_item(true);

    assert_eq!(store.local_using_item_use_effects(), Some(default_effects));

    let mut patched = item_stack(99, 1);
    patched.component_patch.added = 1;
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
fn local_using_item_use_effects_removed_component_falls_back_to_vanilla_default() {
    let mut store = WorldStore::new();
    store.set_default_item_use_effects(BTreeMap::from([(
        42,
        ItemUseEffects {
            can_sprint: true,
            interact_vibrations: false,
            speed_multiplier: 1.0,
        },
    )]));
    let mut item = item_stack(42, 1);
    item.component_patch.removed_type_ids = vec![VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID];
    store.apply_set_player_inventory(ProtocolSetPlayerInventory { slot: 0, item });
    store.set_local_using_item(true);

    assert_eq!(
        store.local_using_item_use_effects(),
        Some(ItemUseEffects::default())
    );
}

#[test]
fn local_using_item_item_id_reads_selected_hand() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 1),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 40,
        item: item_stack(99, 1),
    });

    assert_eq!(store.local_using_item_item_id(), None);

    store.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    assert_eq!(store.local_using_item_item_id(), Some(42));

    store.set_local_using_item_with_hand(true, InteractionHand::OffHand);
    assert_eq!(store.local_using_item_item_id(), Some(99));

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 40,
        item: ProtocolItemStackSummary::empty(),
    });
    assert_eq!(store.local_using_item_item_id(), None);
}

#[test]
fn drop_local_selected_hotbar_item_drops_one_and_updates_menu_view() {
    let mut store = WorldStore::new();
    assert!(store.set_local_selected_hotbar_slot(2));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 2,
        item: item_stack(42, 3),
    });

    assert!(store.drop_local_selected_hotbar_item(false));

    assert_eq!(player_slot_item(&store, 2), item_stack(42, 2));
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START + 2),
        item_stack(42, 2)
    );
}

#[test]
fn drop_local_selected_hotbar_item_drops_stack_and_reports_empty_slots() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(99, 1),
    });

    assert!(store.drop_local_selected_hotbar_item(true));

    assert_eq!(
        player_slot_item(&store, 0),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
        ProtocolItemStackSummary::empty()
    );
    assert!(!store.drop_local_selected_hotbar_item(true));
}

#[test]
fn local_player_usable_glider_applies_default_components_slot_and_damage() {
    let mut store = WorldStore::new();
    store.set_default_item_component_type_ids(BTreeMap::from([(
        42,
        BTreeSet::from([
            VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID,
            VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
            VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
            VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
        ]),
    )]));
    store.set_default_item_equipment_slots(BTreeMap::from([(42, ItemEquipmentSlot::Chest)]));
    store.set_default_item_max_damage(BTreeMap::from([(42, 432)]));

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_CHEST_SLOT,
        item: item_stack(42, 1),
    });
    assert!(store.local_player_has_usable_glider());

    let mut damaged = item_stack(42, 1);
    damaged.component_patch.added = 1;
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

    damaged.component_patch.added = 2;
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
    store.set_default_item_component_type_ids(BTreeMap::from([(
        43,
        BTreeSet::from([
            VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID,
            VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
        ]),
    )]));
    store.set_default_item_equipment_slots(BTreeMap::from([(43, ItemEquipmentSlot::Chest)]));

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: item_stack(43, 1),
    });
    assert!(!store.local_player_has_usable_glider());

    let mut offhand = item_stack(43, 1);
    offhand.component_patch.added = 1;
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
fn hotbar_item_states_include_local_bundle_selection() {
    let mut store = WorldStore::new();

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 4,
        item: bundle_item_stack(10, 1, 2),
    });
    assert!(store.apply_local_select_bundle_item(4, 1));

    let hotbar = store.inventory().hotbar_item_states();
    assert_eq!(hotbar[4].item, bundle_item_stack(10, 1, 2));
    assert_eq!(hotbar[4].local_selected_bundle_item_index(), Some(1));

    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 0,
        state_id: 1,
        items: (0..45)
            .map(|slot| {
                if slot == 40 {
                    bundle_item_stack(20, 1, 2)
                } else {
                    ProtocolItemStackSummary::empty()
                }
            })
            .collect(),
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert!(store.open_local_inventory());
    assert!(store.apply_local_select_bundle_item(40, 0));

    let hotbar = store.inventory().hotbar_item_states();
    assert_eq!(hotbar[4].item, bundle_item_stack(20, 1, 2));
    assert_eq!(hotbar[4].local_selected_bundle_item_index(), Some(0));
    assert_eq!(
        store.inventory().hotbar_items()[4],
        bundle_item_stack(20, 1, 2)
    );
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
fn local_bundle_selection_is_cleared_when_player_slot_item_is_replaced() {
    let mut store = WorldStore::new();

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 4,
        item: bundle_item_stack(42, 1, 4),
    });
    assert!(store.apply_local_select_bundle_item(4, 2));

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 4,
        item: bundle_item_stack(43, 1, 3),
    });
    assert_eq!(
        player_slot_selection(&store, 4),
        NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
    );

    assert!(store.apply_local_select_bundle_item(4, 1));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 4,
        item: bundle_item_stack(44, 1, 2),
    });
    assert_eq!(
        player_slot_selection(&store, 4),
        NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
    );

    assert!(store.apply_local_select_bundle_item(4, 1));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 4,
        item: item_stack(45, 1),
    });
    assert_eq!(
        player_slot_selection(&store, 4),
        NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
    );
    assert!(!store.apply_local_select_bundle_item(4, 0));
}

#[test]
fn local_bundle_selection_applies_to_open_container_slots() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 1,
        item: bundle_item_stack(99, 1, 2),
    });
    open_test_container(&mut store, 7);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 1,
        items: vec![item_stack(42, 1), bundle_item_stack(43, 1, 2)],
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert!(store.apply_local_select_bundle_item(1, 1));
    assert_eq!(container_slot_selection(&store, 1), 1);
    assert_eq!(
        player_slot_selection(&store, 1),
        NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
    );

    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 2,
        items: vec![item_stack(42, 1), bundle_item_stack(43, 1, 2)],
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert_eq!(
        container_slot_selection(&store, 1),
        NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
    );
    assert!(store.apply_local_select_bundle_item(1, 1));
    assert_eq!(container_slot_selection(&store, 1), 1);

    assert!(!store.apply_local_select_bundle_item(0, 0));
    assert_eq!(
        container_slot_selection(&store, 0),
        NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
    );

    assert!(!store.apply_local_select_bundle_item(99, 0));

    store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 7,
        state_id: 2,
        slot: 1,
        item: bundle_item_stack(44, 1, 1),
    });
    assert_eq!(
        container_slot_selection(&store, 1),
        NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
    );

    assert!(store.apply_local_select_bundle_item(1, 0));
    assert_eq!(container_slot_selection(&store, 1), 0);
}

#[test]
fn local_bundle_selection_fields_default_when_deserializing_old_slots() {
    let player_slot: InventorySlot = serde_json::from_value(serde_json::json!({
        "slot": 4,
        "item": item_stack(42, 1),
    }))
    .unwrap();
    assert_eq!(
        player_slot.local_selected_bundle_item_index,
        NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
    );

    let container_slot: ContainerSlot = serde_json::from_value(serde_json::json!({
        "slot": 4,
        "item": item_stack(42, 1),
    }))
    .unwrap();
    assert_eq!(
        container_slot.local_selected_bundle_item_index,
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
fn merchant_offer_scroll_offset_clamps_to_visible_window() {
    let mut store = WorldStore::new();
    assert!(!store.scroll_local_merchant_offers(1));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    assert!(store.apply_merchant_offers(merchant_offers(7, 8)));

    assert!(store.scroll_local_merchant_offers(1));
    assert_eq!(
        store
            .inventory()
            .open_container
            .as_ref()
            .and_then(|container| container.merchant_offers.as_ref())
            .map(|offers| offers.local_scroll_offset),
        Some(1)
    );
    assert!(store.scroll_local_merchant_offers(1));
    assert_eq!(
        store
            .inventory()
            .open_container
            .as_ref()
            .and_then(|container| container.merchant_offers.as_ref())
            .map(|offers| offers.local_scroll_offset),
        Some(1)
    );
    assert!(store.scroll_local_merchant_offers(-1));
    assert_eq!(
        store
            .inventory()
            .open_container
            .as_ref()
            .and_then(|container| container.merchant_offers.as_ref())
            .map(|offers| offers.local_scroll_offset),
        Some(0)
    );
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
    components.added = 1;
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
fn build_container_click_slot_uses_open_container_state_and_cursor_item() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items: vec![ProtocolItemStackSummary::empty(), item_stack(42, 3)],
        carried_item: item_stack(99, 1),
    });

    let click = store
        .build_container_click_slot(ContainerClickSlotRequest {
            slot_num: 1,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(click.container_id, 7);
    assert_eq!(click.state_id, 13);
    assert_eq!(click.slot_num, 1);
    assert_eq!(click.button_num, 0);
    assert_eq!(click.input, ProtocolContainerInput::Pickup);
    assert!(click.changed_slots.is_empty());
    assert_eq!(
        click.carried_item,
        ProtocolHashedStack::Item(ProtocolHashedItemStack {
            item_id: 99,
            count: 1,
            components: ProtocolHashedComponentPatch::default(),
        })
    );
}

#[test]
fn build_container_click_slot_allows_vanilla_outside_slots() {
    let mut store = WorldStore::new();
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 0,
        state_id: 4,
        items: vec![item_stack(42, 1)],
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert!(store.open_local_inventory());

    let outside_click = store
        .build_container_click_slot(ContainerClickSlotRequest {
            slot_num: -999,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    let carried_click = store
        .build_container_click_slot(ContainerClickSlotRequest {
            slot_num: -1,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(outside_click.slot_num, -999);
    assert_eq!(carried_click.slot_num, -1);
}

#[test]
fn build_container_click_slot_rejects_missing_container_invalid_slot_and_unhashable_carried_item() {
    let mut store = WorldStore::new();
    assert_eq!(
        store
            .build_container_click_slot(ContainerClickSlotRequest {
                slot_num: 0,
                button_num: 0,
                input: ProtocolContainerInput::Pickup,
            })
            .unwrap_err(),
        ContainerClickBuildError::NoOpenContainer
    );

    open_test_container(&mut store, 7);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items: vec![item_stack(42, 3)],
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert_eq!(
        store
            .build_container_click_slot(ContainerClickSlotRequest {
                slot_num: 5,
                button_num: 0,
                input: ProtocolContainerInput::Pickup,
            })
            .unwrap_err(),
        ContainerClickBuildError::InvalidSlot(5)
    );

    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack_with_component_summary(99, 1, 10),
    });
    assert_eq!(
        store
            .build_container_click_slot(ContainerClickSlotRequest {
                slot_num: 0,
                button_num: 0,
                input: ProtocolContainerInput::Pickup,
            })
            .unwrap_err(),
        ContainerClickBuildError::UnhashableCarriedItem
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
fn apply_local_beacon_pickup_enforces_payment_tag_and_single_item_limit() {
    let mut store = WorldStore::new();
    apply_item_tags(&mut store, vec![(BEACON_PAYMENT_ITEM_TAG, vec![42])]);
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
        title: bbb_protocol::RichTextComponent::plain("Beacon"),
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items: vec![ProtocolItemStackSummary::empty(); BEACON_TOTAL_SLOT_COUNT as usize],
        carried_item: item_stack(43, 3),
    });

    let rejected = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BEACON_PAYMENT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(rejected.changed_slots, BTreeMap::new());
    assert_eq!(rejected.carried_item, hashed_item_stack(43, 3));
    assert_eq!(
        open_container_slot_item(&store, BEACON_PAYMENT_SLOT),
        ProtocolItemStackSummary::empty()
    );

    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(42, 3),
    });
    let placed = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BEACON_PAYMENT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(
        placed.changed_slots,
        BTreeMap::from([(BEACON_PAYMENT_SLOT, hashed_item_stack(42, 1))])
    );
    assert_eq!(placed.carried_item, hashed_item_stack(42, 2));
    assert_eq!(
        open_container_slot_item(&store, BEACON_PAYMENT_SLOT),
        item_stack(42, 1)
    );
    assert_eq!(store.inventory().cursor_item, item_stack(42, 2));
}

#[test]
fn apply_local_enchantment_pickup_enforces_input_and_lapis_slot_rules() {
    let mut store = WorldStore::new();
    store.set_enchantment_lapis_lazuli_item_ids(BTreeSet::from([43]));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ENCHANTMENT_ID,
        title: bbb_protocol::RichTextComponent::plain("Enchanting Table"),
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items: vec![ProtocolItemStackSummary::empty(); ENCHANTMENT_TOTAL_SLOT_COUNT as usize],
        carried_item: item_stack(42, 3),
    });

    let input_place = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ENCHANTMENT_INPUT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(
        input_place.changed_slots,
        BTreeMap::from([(ENCHANTMENT_INPUT_SLOT, hashed_item_stack(42, 1))])
    );
    assert_eq!(input_place.carried_item, hashed_item_stack(42, 2));

    let rejected_lapis = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ENCHANTMENT_LAPIS_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(rejected_lapis.changed_slots, BTreeMap::new());
    assert_eq!(rejected_lapis.carried_item, hashed_item_stack(42, 2));

    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(43, 2),
    });
    let lapis_place = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ENCHANTMENT_LAPIS_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(
        lapis_place.changed_slots,
        BTreeMap::from([(ENCHANTMENT_LAPIS_SLOT, hashed_item_stack(43, 2))])
    );
    assert_eq!(lapis_place.carried_item, ProtocolHashedStack::Empty);
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
    store.set_default_damageable_item_ids(BTreeSet::from([48]));
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
    let damageable = item_stack(48, 2);
    let banner = item_stack(49, 2);
    let dye = item_stack(50, 2);
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
            VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE,
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
fn apply_local_container_click_slot_hashes_removed_only_component_patch() {
    let mut store = WorldStore::new();
    open_test_container(&mut store, 7);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items: vec![ProtocolItemStackSummary::empty()],
        carried_item: item_stack_with_removed_component_summary(
            42,
            3,
            VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
        ),
    });

    let place = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(
        place.changed_slots,
        BTreeMap::from([(
            0,
            hashed_removed_component_item_stack(42, 3, VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID)
        )])
    );
    assert_eq!(place.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, 0),
        item_stack_with_removed_component_summary(42, 3, VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID)
    );
    assert_eq!(
        store.inventory().cursor_item,
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_container_click_slot_hashes_integer_component_patches() {
    for (component_type_id, value) in [
        (VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID, 16),
        (VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID, 250),
        (VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID, 7),
        (VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID, 7),
    ] {
        let mut store = WorldStore::new();
        let patched_stack = item_stack_with_int_component_summary(42, 1, component_type_id, value);
        open_test_container(&mut store, 7);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 13,
            items: vec![ProtocolItemStackSummary::empty()],
            carried_item: patched_stack.clone(),
        });

        let place = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 0,
                button_num: 0,
                input: ProtocolContainerInput::Pickup,
            })
            .unwrap();

        assert_eq!(
            place.changed_slots,
            BTreeMap::from([(
                0,
                hashed_int_component_item_stack(42, 1, component_type_id, value)
            )])
        );
        assert_eq!(place.carried_item, ProtocolHashedStack::Empty);
        assert_eq!(open_container_slot_item(&store, 0), patched_stack);
    }
}

#[test]
fn apply_local_container_click_slot_hashes_multiple_integer_and_removed_component_patches() {
    let mut store = WorldStore::new();
    let mut patched_stack = item_stack(42, 1);
    patched_stack.component_patch.added = 3;
    patched_stack.component_patch.added_type_ids = vec![
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
    ];
    patched_stack.component_patch.max_stack_size = Some(16);
    patched_stack.component_patch.max_damage = Some(250);
    patched_stack.component_patch.damage = Some(7);
    patched_stack.component_patch.removed_type_ids = vec![VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID];
    open_test_container(&mut store, 7);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items: vec![ProtocolItemStackSummary::empty()],
        carried_item: patched_stack.clone(),
    });

    let place = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(
        place.changed_slots,
        BTreeMap::from([(
            0,
            ProtocolHashedStack::Item(ProtocolHashedItemStack {
                item_id: 42,
                count: 1,
                components: ProtocolHashedComponentPatch {
                    added_components: BTreeMap::from([
                        (
                            VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
                            hash_ops_crc32c_int(16)
                        ),
                        (
                            VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
                            hash_ops_crc32c_int(250)
                        ),
                        (
                            VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
                            hash_ops_crc32c_int(7)
                        ),
                    ]),
                    removed_components: BTreeSet::from([VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID]),
                },
            })
        )])
    );
    assert_eq!(place.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(open_container_slot_item(&store, 0), patched_stack);
}

#[test]
fn apply_local_container_click_slot_rejects_incomplete_integer_component_patch() {
    let mut store = WorldStore::new();
    let mut patched_stack = item_stack(42, 1);
    patched_stack.component_patch.added = 1;
    patched_stack
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID);
    open_test_container(&mut store, 7);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items: vec![ProtocolItemStackSummary::empty()],
        carried_item: patched_stack,
    });

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        }),
        Err(ContainerClickBuildError::UnhashableChangedSlot(0))
    );
}

#[test]
fn apply_local_container_click_slot_hashes_profile_component_patch() {
    let mut store = WorldStore::new();
    open_test_container(&mut store, 7);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items: vec![profiled_item_stack(42, 1)],
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let pickup = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(
        pickup.changed_slots,
        BTreeMap::from([(0, ProtocolHashedStack::Empty)])
    );
    let ProtocolHashedStack::Item(carried) = pickup.carried_item else {
        panic!("profiled carried stack must remain an item");
    };
    assert_eq!(carried.item_id, 42);
    assert_eq!(carried.count, 1);
    assert_eq!(
        carried
            .components
            .added_components
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        vec![VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID]
    );
    assert!(carried.components.removed_components.is_empty());
}

#[test]
fn apply_local_container_click_slot_supports_secondary_pickup_place_and_outside_drop() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 5),
    });
    assert!(store.open_local_inventory());

    let half_pickup = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_HOTBAR_START,
            button_num: 1,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(
        half_pickup.changed_slots,
        BTreeMap::from([(
            INVENTORY_MENU_HOTBAR_START,
            ProtocolHashedStack::Item(ProtocolHashedItemStack {
                item_id: 42,
                count: 2,
                components: ProtocolHashedComponentPatch::default(),
            })
        )])
    );
    assert_eq!(
        half_pickup.carried_item,
        ProtocolHashedStack::Item(ProtocolHashedItemStack {
            item_id: 42,
            count: 3,
            components: ProtocolHashedComponentPatch::default(),
        })
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
        item_stack(42, 2)
    );
    assert_eq!(player_slot_item(&store, 0), item_stack(42, 2));
    assert_eq!(store.inventory().cursor_item, item_stack(42, 3));

    let single_place = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 10,
            button_num: 1,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(
        single_place.changed_slots,
        BTreeMap::from([(
            10,
            ProtocolHashedStack::Item(ProtocolHashedItemStack {
                item_id: 42,
                count: 1,
                components: ProtocolHashedComponentPatch::default(),
            })
        )])
    );
    assert_eq!(
        single_place.carried_item,
        ProtocolHashedStack::Item(ProtocolHashedItemStack {
            item_id: 42,
            count: 2,
            components: ProtocolHashedComponentPatch::default(),
        })
    );
    assert_eq!(inventory_menu_slot_item(&store, 10), item_stack(42, 1));
    assert_eq!(player_slot_item(&store, 10), item_stack(42, 1));
    assert_eq!(store.inventory().cursor_item, item_stack(42, 2));

    let drop_one = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: -999,
            button_num: 1,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert!(drop_one.changed_slots.is_empty());
    assert_eq!(
        drop_one.carried_item,
        ProtocolHashedStack::Item(ProtocolHashedItemStack {
            item_id: 42,
            count: 1,
            components: ProtocolHashedComponentPatch::default(),
        })
    );
    assert_eq!(store.inventory().cursor_item, item_stack(42, 1));

    let drop_remaining = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: -999,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert!(drop_remaining.changed_slots.is_empty());
    assert_eq!(drop_remaining.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        store.inventory().cursor_item,
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_container_click_slot_uses_default_item_max_stack_sizes() {
    let mut store = WorldStore::new();
    store.set_default_item_max_stack_sizes(BTreeMap::from([(42, 16)]));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 2),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 1,
        item: item_stack(42, 15),
    });
    assert!(store.open_local_inventory());

    store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    let merge = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_HOTBAR_START + 1,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(
        merge.changed_slots,
        BTreeMap::from([(
            INVENTORY_MENU_HOTBAR_START + 1,
            ProtocolHashedStack::Item(ProtocolHashedItemStack {
                item_id: 42,
                count: 16,
                components: ProtocolHashedComponentPatch::default(),
            })
        )])
    );
    assert_eq!(
        merge.carried_item,
        ProtocolHashedStack::Item(ProtocolHashedItemStack {
            item_id: 42,
            count: 1,
            components: ProtocolHashedComponentPatch::default(),
        })
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START + 1),
        item_stack(42, 16)
    );
    assert_eq!(store.inventory().cursor_item, item_stack(42, 1));
}

#[test]
fn apply_local_container_click_slot_respects_unstackable_and_local_slot_limits() {
    let mut store = WorldStore::new();
    store.set_default_item_max_stack_sizes(BTreeMap::from([(42, 1), (43, 64)]));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 1),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 1,
        item: item_stack(42, 1),
    });
    assert!(store.open_local_inventory());

    store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    let blocked_merge = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_HOTBAR_START + 1,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert!(blocked_merge.changed_slots.is_empty());
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START + 1),
        item_stack(42, 1)
    );
    assert_eq!(store.inventory().cursor_item, item_stack(42, 1));

    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(43, 3),
    });
    let result_slot_place = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert!(result_slot_place.changed_slots.is_empty());
    assert_eq!(store.inventory().cursor_item, item_stack(43, 3));

    let armor_place = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 5,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(
        armor_place.changed_slots,
        BTreeMap::from([(
            5,
            ProtocolHashedStack::Item(ProtocolHashedItemStack {
                item_id: 43,
                count: 1,
                components: ProtocolHashedComponentPatch::default(),
            })
        )])
    );
    assert_eq!(inventory_menu_slot_item(&store, 5), item_stack(43, 1));
    assert_eq!(store.inventory().cursor_item, item_stack(43, 2));
}

#[test]
fn apply_local_container_pickup_all_collects_matching_stacks() {
    let mut store = WorldStore::new();
    store.set_default_item_max_stack_sizes(BTreeMap::from([(42, 16), (43, 16)]));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 9,
        item: item_stack(42, 3),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 10,
        item: item_stack(42, 4),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 11,
        item: item_stack(42, 16),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 12,
        item: item_stack(43, 5),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 14,
        item: item_stack_with_component_summary(42, 6, 7),
    });
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(42, 8),
    });
    assert!(store.open_local_inventory());

    let pickup_all = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 13,
            button_num: 0,
            input: ProtocolContainerInput::PickupAll,
        })
        .unwrap();

    assert_eq!(pickup_all.container_id, INVENTORY_MENU_CONTAINER_ID);
    assert_eq!(pickup_all.input, ProtocolContainerInput::PickupAll);
    assert_eq!(
        pickup_all.changed_slots,
        BTreeMap::from([
            (INVENTORY_MENU_MAIN_START, ProtocolHashedStack::Empty),
            (INVENTORY_MENU_MAIN_START + 1, ProtocolHashedStack::Empty),
            (INVENTORY_MENU_MAIN_START + 2, hashed_item_stack(42, 15)),
        ])
    );
    assert_eq!(pickup_all.carried_item, hashed_item_stack(42, 16));
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START + 1),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START + 2),
        item_stack(42, 15)
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START + 3),
        item_stack(43, 5)
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START + 5),
        item_stack_with_component_summary(42, 6, 7)
    );
    assert_eq!(store.inventory().cursor_item, item_stack(42, 16));
}

#[test]
fn apply_local_container_pickup_all_excludes_crafting_result_slot() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    let mut items =
        vec![ProtocolItemStackSummary::empty(); CRAFTING_MENU_TOTAL_SLOT_COUNT as usize];
    items[CRAFTING_MENU_RESULT_SLOT as usize] = item_stack(42, 1);
    items[CRAFTING_MENU_PLAYER_MAIN_START as usize] = item_stack(42, 2);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items,
        carried_item: item_stack(42, 1),
    });

    let pickup_all = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: CRAFTING_MENU_PLAYER_MAIN_START + 1,
            button_num: 0,
            input: ProtocolContainerInput::PickupAll,
        })
        .unwrap();

    assert_eq!(
        pickup_all.changed_slots,
        BTreeMap::from([(CRAFTING_MENU_PLAYER_MAIN_START, ProtocolHashedStack::Empty)])
    );
    assert_eq!(pickup_all.carried_item, hashed_item_stack(42, 3));
    assert_eq!(
        open_container_slot_item(&store, CRAFTING_MENU_RESULT_SLOT),
        item_stack(42, 1)
    );
}

#[test]
fn apply_local_container_pickup_all_button_one_collects_in_reverse_order() {
    let mut store = WorldStore::new();
    store.set_default_item_max_stack_sizes(BTreeMap::from([(42, 16)]));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 9,
        item: item_stack(42, 4),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 8,
        item: item_stack(42, 4),
    });
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(42, 14),
    });
    assert!(store.open_local_inventory());

    let pickup_all = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_MAIN_START + 1,
            button_num: 1,
            input: ProtocolContainerInput::PickupAll,
        })
        .unwrap();

    assert_eq!(
        pickup_all.changed_slots,
        BTreeMap::from([(INVENTORY_MENU_HOTBAR_END - 1, hashed_item_stack(42, 2))])
    );
    assert_eq!(pickup_all.carried_item, hashed_item_stack(42, 16));
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
        item_stack(42, 4)
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_END - 1),
        item_stack(42, 2)
    );
    assert_eq!(store.inventory().cursor_item, item_stack(42, 16));
}

#[test]
fn apply_local_container_pickup_all_noops_without_eligible_click() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 9,
        item: item_stack(42, 3),
    });
    assert!(store.open_local_inventory());

    let empty_cursor = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_MAIN_START + 1,
            button_num: 0,
            input: ProtocolContainerInput::PickupAll,
        })
        .unwrap();
    assert!(empty_cursor.changed_slots.is_empty());
    assert_eq!(empty_cursor.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
        item_stack(42, 3)
    );

    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(42, 5),
    });
    let clicked_item = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::PickupAll,
        })
        .unwrap();
    assert!(clicked_item.changed_slots.is_empty());
    assert_eq!(clicked_item.carried_item, hashed_item_stack(42, 5));
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
        item_stack(42, 3)
    );

    for slot_num in [-999, -1] {
        let outside = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num,
                button_num: 0,
                input: ProtocolContainerInput::PickupAll,
            })
            .unwrap();
        assert!(outside.changed_slots.is_empty());
        assert_eq!(outside.carried_item, hashed_item_stack(42, 5));
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
            item_stack(42, 3)
        );
    }
}

#[test]
fn apply_local_container_pickup_all_predicts_generic_menu() {
    let mut store = WorldStore::new();
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(42, 5),
    });
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X2_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 54];
    items[1] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items,
        carried_item: item_stack(42, 5),
    });

    let pickup_all = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::PickupAll,
        })
        .unwrap();

    assert_eq!(
        pickup_all.changed_slots,
        BTreeMap::from([(1, ProtocolHashedStack::Empty)])
    );
    assert_eq!(pickup_all.carried_item, hashed_item_stack(42, 8));
    assert_eq!(
        store.inventory().open_container.as_ref().unwrap().slots[1].item,
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(store.inventory().cursor_item, item_stack(42, 8));
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
fn apply_local_merchant_pickup_all_honors_menu_wide_exclusion() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); MERCHANT_TOTAL_SLOT_COUNT as usize];
    items[MERCHANT_PAYMENT_SLOT_1 as usize] = item_stack(42, 2);
    items[MERCHANT_PLAYER_MAIN_START as usize] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items,
        carried_item: item_stack(42, 1),
    });

    let pickup_all = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_PLAYER_MAIN_START + 1,
            button_num: 0,
            input: ProtocolContainerInput::PickupAll,
        })
        .unwrap();

    assert_eq!(pickup_all.changed_slots, BTreeMap::new());
    assert_eq!(pickup_all.carried_item, hashed_item_stack(42, 1));
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_1),
        item_stack(42, 2)
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PLAYER_MAIN_START),
        item_stack(42, 3)
    );
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
fn apply_local_container_quick_craft_right_drag_places_one_per_slot() {
    let mut store = WorldStore::new();
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(42, 5),
    });
    assert!(store.open_local_inventory());

    store
        .apply_local_container_click_slot(quick_craft_request(
            -999,
            QUICKCRAFT_HEADER_START,
            QUICKCRAFT_TYPE_GREEDY,
        ))
        .unwrap();
    store
        .apply_local_container_click_slot(quick_craft_request(
            INVENTORY_MENU_MAIN_START,
            QUICKCRAFT_HEADER_CONTINUE,
            QUICKCRAFT_TYPE_GREEDY,
        ))
        .unwrap();
    store
        .apply_local_container_click_slot(quick_craft_request(
            INVENTORY_MENU_MAIN_START + 1,
            QUICKCRAFT_HEADER_CONTINUE,
            QUICKCRAFT_TYPE_GREEDY,
        ))
        .unwrap();
    let finish = store
        .apply_local_container_click_slot(quick_craft_request(
            -999,
            QUICKCRAFT_HEADER_END,
            QUICKCRAFT_TYPE_GREEDY,
        ))
        .unwrap();

    assert_eq!(
        finish.changed_slots,
        BTreeMap::from([
            (INVENTORY_MENU_MAIN_START, hashed_item_stack(42, 1)),
            (INVENTORY_MENU_MAIN_START + 1, hashed_item_stack(42, 1)),
        ])
    );
    assert_eq!(finish.carried_item, hashed_item_stack(42, 3));
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
        item_stack(42, 1)
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START + 1),
        item_stack(42, 1)
    );
    assert_eq!(store.inventory().cursor_item, item_stack(42, 3));
}

#[test]
fn apply_local_container_quick_craft_single_slot_finish_uses_pickup_semantics() {
    let mut store = WorldStore::new();
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(42, 5),
    });
    assert!(store.open_local_inventory());

    store
        .apply_local_container_click_slot(quick_craft_request(
            -999,
            QUICKCRAFT_HEADER_START,
            QUICKCRAFT_TYPE_GREEDY,
        ))
        .unwrap();
    store
        .apply_local_container_click_slot(quick_craft_request(
            INVENTORY_MENU_MAIN_START,
            QUICKCRAFT_HEADER_CONTINUE,
            QUICKCRAFT_TYPE_GREEDY,
        ))
        .unwrap();
    let finish = store
        .apply_local_container_click_slot(quick_craft_request(
            -999,
            QUICKCRAFT_HEADER_END,
            QUICKCRAFT_TYPE_GREEDY,
        ))
        .unwrap();

    assert_eq!(
        finish.changed_slots,
        BTreeMap::from([(INVENTORY_MENU_MAIN_START, hashed_item_stack(42, 1))])
    );
    assert_eq!(finish.carried_item, hashed_item_stack(42, 4));
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
        item_stack(42, 1)
    );
    assert_eq!(store.inventory().cursor_item, item_stack(42, 4));
}

#[test]
fn apply_local_container_quick_craft_invalid_type_or_order_resets_without_corruption() {
    let mut store = WorldStore::new();
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(42, 8),
    });
    assert!(store.open_local_inventory());

    let clone_start = store
        .apply_local_container_click_slot(quick_craft_request(
            -999,
            QUICKCRAFT_HEADER_START,
            QUICKCRAFT_TYPE_CLONE,
        ))
        .unwrap();
    assert!(clone_start.changed_slots.is_empty());
    assert_eq!(clone_start.carried_item, hashed_item_stack(42, 8));

    let continue_without_start = store
        .apply_local_container_click_slot(quick_craft_request(
            INVENTORY_MENU_MAIN_START,
            QUICKCRAFT_HEADER_CONTINUE,
            QUICKCRAFT_TYPE_CHARITABLE,
        ))
        .unwrap();
    assert!(continue_without_start.changed_slots.is_empty());
    assert_eq!(
        continue_without_start.carried_item,
        hashed_item_stack(42, 8)
    );

    store
        .apply_local_container_click_slot(quick_craft_request(
            -999,
            QUICKCRAFT_HEADER_START,
            QUICKCRAFT_TYPE_CHARITABLE,
        ))
        .unwrap();
    let pickup_while_active = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert!(pickup_while_active.changed_slots.is_empty());
    assert_eq!(pickup_while_active.carried_item, hashed_item_stack(42, 8));

    let stale_continue = store
        .apply_local_container_click_slot(quick_craft_request(
            INVENTORY_MENU_MAIN_START,
            QUICKCRAFT_HEADER_CONTINUE,
            QUICKCRAFT_TYPE_CHARITABLE,
        ))
        .unwrap();
    assert!(stale_continue.changed_slots.is_empty());
    assert_eq!(stale_continue.carried_item, hashed_item_stack(42, 8));
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(store.inventory().cursor_item, item_stack(42, 8));
}

#[test]
fn apply_local_container_quick_craft_predicts_generic_menu() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X2_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items: vec![ProtocolItemStackSummary::empty(); 54],
        carried_item: item_stack(42, 8),
    });

    store
        .apply_local_container_click_slot(quick_craft_request(
            -999,
            QUICKCRAFT_HEADER_START,
            QUICKCRAFT_TYPE_CHARITABLE,
        ))
        .unwrap();
    store
        .apply_local_container_click_slot(quick_craft_request(
            0,
            QUICKCRAFT_HEADER_CONTINUE,
            QUICKCRAFT_TYPE_CHARITABLE,
        ))
        .unwrap();
    store
        .apply_local_container_click_slot(quick_craft_request(
            1,
            QUICKCRAFT_HEADER_CONTINUE,
            QUICKCRAFT_TYPE_CHARITABLE,
        ))
        .unwrap();
    let finish = store
        .apply_local_container_click_slot(quick_craft_request(
            -999,
            QUICKCRAFT_HEADER_END,
            QUICKCRAFT_TYPE_CHARITABLE,
        ))
        .unwrap();

    assert_eq!(
        finish.changed_slots,
        BTreeMap::from([(0, hashed_item_stack(42, 4)), (1, hashed_item_stack(42, 4)),])
    );
    assert_eq!(finish.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(open_container_slot_item(&store, 0), item_stack(42, 4));
    assert_eq!(open_container_slot_item(&store, 1), item_stack(42, 4));
    assert_eq!(
        store.inventory().cursor_item,
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_brewing_stand_quick_craft_only_selects_matching_bottle_slots() {
    let mut store = WorldStore::new();
    store.set_brewing_potion_item_ids(BTreeSet::from([42]));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BREWING_STAND_ID,
        title: bbb_protocol::RichTextComponent::plain("Brewing Stand"),
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items: vec![ProtocolItemStackSummary::empty(); BREWING_STAND_TOTAL_SLOT_COUNT as usize],
        carried_item: item_stack(42, 3),
    });

    store
        .apply_local_container_click_slot(quick_craft_request(
            -999,
            QUICKCRAFT_HEADER_START,
            QUICKCRAFT_TYPE_CHARITABLE,
        ))
        .unwrap();
    for slot_num in [
        BREWING_STAND_BOTTLE_SLOT_START,
        BREWING_STAND_BOTTLE_SLOT_START + 1,
        BREWING_STAND_INGREDIENT_SLOT,
    ] {
        store
            .apply_local_container_click_slot(quick_craft_request(
                slot_num,
                QUICKCRAFT_HEADER_CONTINUE,
                QUICKCRAFT_TYPE_CHARITABLE,
            ))
            .unwrap();
    }
    let finish = store
        .apply_local_container_click_slot(quick_craft_request(
            -999,
            QUICKCRAFT_HEADER_END,
            QUICKCRAFT_TYPE_CHARITABLE,
        ))
        .unwrap();

    assert_eq!(
        finish.changed_slots,
        BTreeMap::from([
            (BREWING_STAND_BOTTLE_SLOT_START, hashed_item_stack(42, 1)),
            (
                BREWING_STAND_BOTTLE_SLOT_START + 1,
                hashed_item_stack(42, 1)
            ),
        ])
    );
    assert_eq!(finish.carried_item, hashed_item_stack(42, 1));
    assert_eq!(
        open_container_slot_item(&store, BREWING_STAND_BOTTLE_SLOT_START),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, BREWING_STAND_BOTTLE_SLOT_START + 1),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, BREWING_STAND_INGREDIENT_SLOT),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_crafter_quick_craft_skips_disabled_grid_slots() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTER_3X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafter"),
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items: vec![ProtocolItemStackSummary::empty(); CRAFTER_TOTAL_SLOT_COUNT as usize],
        carried_item: item_stack(42, 6),
    });
    store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 7,
        id: 1,
        value: 1,
    });

    store
        .apply_local_container_click_slot(quick_craft_request(
            -999,
            QUICKCRAFT_HEADER_START,
            QUICKCRAFT_TYPE_CHARITABLE,
        ))
        .unwrap();
    for slot_num in [0, 1, 2] {
        store
            .apply_local_container_click_slot(quick_craft_request(
                slot_num,
                QUICKCRAFT_HEADER_CONTINUE,
                QUICKCRAFT_TYPE_CHARITABLE,
            ))
            .unwrap();
    }
    let finish = store
        .apply_local_container_click_slot(quick_craft_request(
            -999,
            QUICKCRAFT_HEADER_END,
            QUICKCRAFT_TYPE_CHARITABLE,
        ))
        .unwrap();

    assert_eq!(
        finish.changed_slots,
        BTreeMap::from([(0, hashed_item_stack(42, 3)), (2, hashed_item_stack(42, 3))])
    );
    assert_eq!(finish.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, 1),
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
fn apply_local_container_quick_move_moves_hotbar_to_inventory() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 3),
    });
    assert!(store.open_local_inventory());

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(quick_move.container_id, INVENTORY_MENU_CONTAINER_ID);
    assert_eq!(quick_move.input, ProtocolContainerInput::QuickMove);
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (INVENTORY_MENU_MAIN_START, hashed_item_stack(42, 3)),
            (INVENTORY_MENU_HOTBAR_START, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(inventory_menu_slot_item(&store, 9), item_stack(42, 3));
    assert_eq!(
        player_slot_item(&store, 0),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(player_slot_item(&store, 9), item_stack(42, 3));
}

#[test]
fn apply_local_container_quick_move_moves_inventory_to_hotbar() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 9,
        item: item_stack(43, 4),
    });
    assert!(store.open_local_inventory());

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_MAIN_START,
            button_num: 1,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (INVENTORY_MENU_MAIN_START, ProtocolHashedStack::Empty),
            (INVENTORY_MENU_HOTBAR_START, hashed_item_stack(43, 4)),
        ])
    );
    assert_eq!(
        inventory_menu_slot_item(&store, 9),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
        item_stack(43, 4)
    );
    assert_eq!(
        player_slot_item(&store, 9),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(player_slot_item(&store, 0), item_stack(43, 4));
}

#[test]
fn apply_local_container_quick_move_merges_then_fills_with_stack_limits() {
    let mut store = WorldStore::new();
    store.set_default_item_max_stack_sizes(BTreeMap::from([(42, 16)]));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 2),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 9,
        item: item_stack(42, 15),
    });
    assert!(store.open_local_inventory());

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (INVENTORY_MENU_MAIN_START, hashed_item_stack(42, 16)),
            (INVENTORY_MENU_MAIN_START + 1, hashed_item_stack(42, 1)),
            (INVENTORY_MENU_HOTBAR_START, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(inventory_menu_slot_item(&store, 9), item_stack(42, 16));
    assert_eq!(inventory_menu_slot_item(&store, 10), item_stack(42, 1));
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(player_slot_item(&store, 9), item_stack(42, 16));
    assert_eq!(player_slot_item(&store, 10), item_stack(42, 1));
    assert_eq!(
        player_slot_item(&store, 0),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_container_quick_move_auto_equips_armor_slot() {
    let mut store = WorldStore::new();
    store.set_default_item_equipment_slots(BTreeMap::from([(42, ItemEquipmentSlot::Chest)]));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 1),
    });
    assert!(store.open_local_inventory());

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (6, hashed_item_stack(42, 1)),
            (INVENTORY_MENU_HOTBAR_START, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(inventory_menu_slot_item(&store, 6), item_stack(42, 1));
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        player_slot_item(&store, VANILLA_PLAYER_INVENTORY_CHEST_SLOT),
        item_stack(42, 1)
    );
    assert_eq!(
        player_slot_item(&store, 0),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_container_quick_move_auto_equips_offhand_slot() {
    let mut store = WorldStore::new();
    store.set_default_item_equipment_slots(BTreeMap::from([(43, ItemEquipmentSlot::OffHand)]));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 9,
        item: item_stack(43, 1),
    });
    assert!(store.open_local_inventory());

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (INVENTORY_MENU_MAIN_START, ProtocolHashedStack::Empty),
            (INVENTORY_MENU_OFFHAND_SLOT, hashed_item_stack(43, 1)),
        ])
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_OFFHAND_SLOT),
        item_stack(43, 1)
    );
    assert_eq!(
        player_slot_item(&store, 9),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        player_slot_item(&store, VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT),
        item_stack(43, 1)
    );
}

#[test]
fn apply_local_container_quick_move_uses_inventory_fallback_when_equipment_slot_is_occupied() {
    let mut store = WorldStore::new();
    store.set_default_item_equipment_slots(BTreeMap::from([(42, ItemEquipmentSlot::Chest)]));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_CHEST_SLOT,
        item: item_stack(99, 1),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 9,
        item: item_stack(42, 1),
    });
    assert!(store.open_local_inventory());

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (INVENTORY_MENU_MAIN_START, ProtocolHashedStack::Empty),
            (INVENTORY_MENU_HOTBAR_START, hashed_item_stack(42, 1)),
        ])
    );
    assert_eq!(inventory_menu_slot_item(&store, 6), item_stack(99, 1));
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
        item_stack(42, 1)
    );
    assert_eq!(
        player_slot_item(&store, VANILLA_PLAYER_INVENTORY_CHEST_SLOT),
        item_stack(99, 1)
    );
    assert_eq!(
        player_slot_item(&store, 9),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(player_slot_item(&store, 0), item_stack(42, 1));
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
fn inventory_menu_crafting_result_quick_move_consumes_inputs_once() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 2);
    items[1] = item_stack(42, 3);
    items[2] = item_stack(43, 3);
    items[4] = item_stack(44, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: INVENTORY_MENU_CONTAINER_ID,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert!(store.open_local_inventory());

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (0, ProtocolHashedStack::Empty),
            (1, hashed_item_stack(42, 2)),
            (2, hashed_item_stack(43, 2)),
            (4, hashed_item_stack(44, 2)),
            (44, hashed_item_stack(90, 2)),
        ])
    );
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        inventory_menu_slot_item(&store, 0),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(inventory_menu_slot_item(&store, 1), item_stack(42, 2));
    assert_eq!(inventory_menu_slot_item(&store, 2), item_stack(43, 2));
    assert_eq!(inventory_menu_slot_item(&store, 4), item_stack(44, 2));
    assert_eq!(inventory_menu_slot_item(&store, 44), item_stack(90, 2));
    assert_eq!(player_slot_item(&store, 8), item_stack(90, 2));
}

#[test]
fn inventory_menu_crafting_result_quick_move_keeps_unmoved_result_remainder() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    store.set_default_item_max_stack_sizes(BTreeMap::from([(90, 1)]));
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 2);
    items[1] = item_stack(42, 2);
    items[2] = item_stack(43, 2);
    items[4] = item_stack(44, 2);
    for slot in 9..44 {
        items[slot] = item_stack(99, 1);
    }
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: INVENTORY_MENU_CONTAINER_ID,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert!(store.open_local_inventory());

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (0, hashed_item_stack(90, 1)),
            (1, hashed_item_stack(42, 1)),
            (2, hashed_item_stack(43, 1)),
            (4, hashed_item_stack(44, 1)),
            (44, hashed_item_stack(90, 1)),
        ])
    );
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(inventory_menu_slot_item(&store, 0), item_stack(90, 1));
    assert_eq!(inventory_menu_slot_item(&store, 1), item_stack(42, 1));
    assert_eq!(inventory_menu_slot_item(&store, 2), item_stack(43, 1));
    assert_eq!(inventory_menu_slot_item(&store, 4), item_stack(44, 1));
    assert_eq!(inventory_menu_slot_item(&store, 43), item_stack(99, 1));
    assert_eq!(inventory_menu_slot_item(&store, 44), item_stack(90, 1));
    assert_eq!(player_slot_item(&store, 7), item_stack(99, 1));
    assert_eq!(player_slot_item(&store, 8), item_stack(90, 1));
}

#[test]
fn inventory_menu_crafting_result_remains_server_owned_after_input_click() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    store.apply_recipe_book_add(ProtocolRecipeBookAdd {
        entries: vec![recipe_book_crafting_entry(
            7,
            CraftingRecipeDisplaySummary::Shaped {
                width: 2,
                height: 1,
                ingredients: vec![slot_display_item(42), slot_display_item(43)],
                result: slot_display_item(91),
                crafting_station: slot_display_empty(),
            },
        )],
        replace: true,
    });
    let mut server_result = component_rich_item_stack();
    server_result.count = 1;
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = server_result.clone();
    items[1] = item_stack(42, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: INVENTORY_MENU_CONTAINER_ID,
        state_id: 12,
        items,
        carried_item: item_stack(43, 1),
    });
    assert!(store.open_local_inventory());

    let pickup = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 2,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(
        pickup.changed_slots,
        BTreeMap::from([(2, hashed_item_stack(43, 1))])
    );
    assert_eq!(pickup.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(inventory_menu_slot_item(&store, 0), server_result);
    assert_eq!(inventory_menu_slot_item(&store, 1), item_stack(42, 1));
    assert_eq!(inventory_menu_slot_item(&store, 2), item_stack(43, 1));
}

#[test]
fn apply_local_container_result_requires_known_crafting_remainders() {
    let mut store = WorldStore::new();
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: INVENTORY_MENU_CONTAINER_ID,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert!(store.open_local_inventory());

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::Pickup
        ))
    );
    assert_eq!(inventory_menu_slot_item(&store, 0), item_stack(90, 1));
    assert_eq!(inventory_menu_slot_item(&store, 1), item_stack(42, 1));
    assert_eq!(
        store.inventory().cursor_item,
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_container_result_places_default_remainder_in_crafting_slot() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::from([(42, 43)]));
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 1);
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
            (1, hashed_item_stack(43, 1))
        ])
    );
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 1));
    assert_eq!(
        inventory_menu_slot_item(&store, 0),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(inventory_menu_slot_item(&store, 1), item_stack(43, 1));
    assert_eq!(store.inventory().cursor_item, item_stack(90, 1));
}

#[test]
fn apply_local_container_result_adds_default_remainder_to_visible_player_inventory() {
    let mut store = WorldStore::new();
    assert!(store.set_local_selected_hotbar_slot(2));
    store.set_default_item_crafting_remainders(BTreeMap::from([(42, 43)]));
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 2);
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
            (36, hashed_item_stack(43, 1)),
        ])
    );
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 1));
    assert_eq!(
        inventory_menu_slot_item(&store, 0),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(inventory_menu_slot_item(&store, 1), item_stack(42, 1));
    assert_eq!(inventory_menu_slot_item(&store, 36), item_stack(43, 1));
    assert_eq!(player_slot_item(&store, 0), item_stack(43, 1));
}

#[test]
fn inventory_menu_crafting_result_drops_default_remainder_when_inventory_is_full() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::from([(42, 43)]));
    store.set_default_item_max_stack_sizes(BTreeMap::from([(43, 1), (99, 1)]));
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 2);
    for slot in 9..45 {
        items[slot] = item_stack(99, 1);
    }
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
            (1, hashed_item_stack(42, 1))
        ])
    );
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 1));
    assert_eq!(
        inventory_menu_slot_item(&store, 0),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(inventory_menu_slot_item(&store, 1), item_stack(42, 1));
    assert_eq!(store.inventory().cursor_item, item_stack(90, 1));
}

#[test]
fn inventory_menu_crafting_result_uses_default_prediction_for_recipe_specific_input() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 2);
    items[5] = item_stack(43, 2);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: INVENTORY_MENU_CONTAINER_ID,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert!(store.open_local_inventory());

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (0, ProtocolHashedStack::Empty),
            (1, hashed_item_stack(42, 1)),
            (44, hashed_item_stack(90, 1)),
        ])
    );
    assert_eq!(
        inventory_menu_slot_item(&store, 0),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(inventory_menu_slot_item(&store, 1), item_stack(42, 1));
    assert_eq!(inventory_menu_slot_item(&store, 5), item_stack(43, 2));
    assert_eq!(inventory_menu_slot_item(&store, 44), item_stack(90, 1));
}

#[test]
fn apply_local_generic_container_quick_move_moves_chest_to_player_reverse() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X6_ID,
        title: bbb_protocol::RichTextComponent::plain("Large Chest"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 90];
    items[0] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(quick_move.container_id, 7);
    assert_eq!(quick_move.state_id, 12);
    assert_eq!(quick_move.input, ProtocolContainerInput::QuickMove);
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (0, ProtocolHashedStack::Empty),
            (89, hashed_item_stack(42, 3))
        ])
    );
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    let slots = &store.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[89].item, item_stack(42, 3));
}

#[test]
fn apply_local_generic_container_quick_move_merges_player_to_chest_forward() {
    let mut store = WorldStore::new();
    store.set_default_item_max_stack_sizes(BTreeMap::from([(42, 16)]));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X2_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 54];
    items[0] = item_stack(42, 15);
    items[18] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 18,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(quick_move.container_id, 7);
    assert_eq!(quick_move.state_id, 13);
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (0, hashed_item_stack(42, 16)),
            (1, hashed_item_stack(42, 2)),
            (18, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    let slots = &store.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, item_stack(42, 16));
    assert_eq!(slots[1].item, item_stack(42, 2));
    assert_eq!(slots[18].item, ProtocolItemStackSummary::empty());
}

#[test]
fn apply_local_generic_3x3_quick_move_moves_dispenser_to_player_reverse() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_3X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Dispenser"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 45];
    items[0] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(quick_move.container_id, 7);
    assert_eq!(quick_move.state_id, 12);
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (0, ProtocolHashedStack::Empty),
            (44, hashed_item_stack(42, 3))
        ])
    );
    let slots = &store.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[44].item, item_stack(42, 3));
}

#[test]
fn apply_local_generic_3x3_quick_move_moves_player_to_dispenser_forward() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_3X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Dispenser"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 45];
    items[9] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 9,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(quick_move.container_id, 7);
    assert_eq!(quick_move.state_id, 13);
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (0, hashed_item_stack(42, 3)),
            (9, ProtocolHashedStack::Empty)
        ])
    );
    let slots = &store.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, item_stack(42, 3));
    assert_eq!(slots[9].item, ProtocolItemStackSummary::empty());
}

#[test]
fn apply_local_crafting_menu_quick_move_moves_grid_to_player_forward() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[1] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 1,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(quick_move.container_id, 7);
    assert_eq!(quick_move.state_id, 12);
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (1, ProtocolHashedStack::Empty),
            (10, hashed_item_stack(42, 3))
        ])
    );
    let slots = &store.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[1].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[10].item, item_stack(42, 3));
}

#[test]
fn apply_local_crafting_menu_quick_move_moves_player_to_grid_then_between_player_ranges() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[10] = item_stack(42, 3);
    items[37] = item_stack(43, 4);
    for slot in 1..10 {
        items[slot] = item_stack(90 + slot as i32, 1);
    }
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let main_to_hotbar = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 10,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        main_to_hotbar.changed_slots,
        BTreeMap::from([
            (10, ProtocolHashedStack::Empty),
            (38, hashed_item_stack(42, 3))
        ])
    );

    let hotbar_to_main = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 37,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        hotbar_to_main.changed_slots,
        BTreeMap::from([
            (10, hashed_item_stack(43, 4)),
            (37, ProtocolHashedStack::Empty)
        ])
    );
    let slots = &store.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[10].item, item_stack(43, 4));
    assert_eq!(slots[37].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[38].item, item_stack(42, 3));
}

#[test]
fn crafting_result_quick_move_consumes_single_inputs() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 1);
    items[5] = item_stack(43, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    let request = ContainerClickSlotRequest {
        slot_num: 0,
        button_num: 0,
        input: ProtocolContainerInput::QuickMove,
    };

    let quick_move = store.apply_local_container_click_slot(request).unwrap();
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (0, ProtocolHashedStack::Empty),
            (1, ProtocolHashedStack::Empty),
            (5, ProtocolHashedStack::Empty),
            (45, hashed_item_stack(90, 1)),
        ])
    );
    let slots = &store.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[1].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[5].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[45].item, item_stack(90, 1));
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

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (0, ProtocolHashedStack::Empty),
            (1, hashed_item_stack(42, 1)),
            (5, hashed_item_stack(43, 1)),
            (45, hashed_item_stack(90, 1)),
        ])
    );
    let slots = &store.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[1].item, item_stack(42, 1));
    assert_eq!(slots[5].item, item_stack(43, 1));
    assert_eq!(slots[45].item, item_stack(90, 1));
}

#[test]
fn crafting_result_pickup_leaves_result_empty_when_inputs_remain() {
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
            (5, hashed_item_stack(43, 1)),
        ])
    );
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 1));
    let slots = &store.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[1].item, item_stack(42, 1));
    assert_eq!(slots[5].item, item_stack(43, 1));
    assert_eq!(store.inventory().cursor_item, item_stack(90, 1));
}

#[test]
fn crafting_result_pickup_consumes_single_inputs_to_cursor() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 1);
    items[5] = item_stack(43, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

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
            (1, ProtocolHashedStack::Empty),
            (5, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 1));
    let slots = &store.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[1].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[5].item, ProtocolItemStackSummary::empty());
    assert_eq!(store.inventory().cursor_item, item_stack(90, 1));
}

#[test]
fn crafting_result_secondary_pickup_keeps_unmoved_server_result_count() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 3);
    items[1] = item_stack(42, 2);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let pickup = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 1,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(
        pickup.changed_slots,
        BTreeMap::from([(0, hashed_item_stack(90, 1)), (1, hashed_item_stack(42, 1)),])
    );
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 2));
    let slots = &store.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, item_stack(90, 1));
    assert_eq!(slots[1].item, item_stack(42, 1));
    assert_eq!(store.inventory().cursor_item, item_stack(90, 2));
}

#[test]
fn crafting_result_places_default_remainder_in_crafting_slot() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::from([(42, 43)]));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

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
            (1, hashed_item_stack(43, 1))
        ])
    );
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 1));
    let slots = &store.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[1].item, item_stack(43, 1));
    assert_eq!(store.inventory().cursor_item, item_stack(90, 1));
}

#[test]
fn crafting_result_inserts_incompatible_default_remainder_into_player_inventory() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::from([(42, 43)]));
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
            (37, hashed_item_stack(43, 1)),
        ])
    );
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 1));
    let slots = &store.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[1].item, item_stack(42, 1));
    assert_eq!(slots[37].item, item_stack(43, 1));
    assert_eq!(store.inventory().cursor_item, item_stack(90, 1));
}

#[test]
fn crafting_result_uses_default_prediction_for_recipe_specific_input() {
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

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (0, ProtocolHashedStack::Empty),
            (1, hashed_item_stack(42, 1)),
            (5, hashed_item_stack(43, 1)),
            (45, hashed_item_stack(90, 1)),
        ])
    );
    let slots = &store.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[1].item, item_stack(42, 1));
    assert_eq!(slots[5].item, item_stack(43, 1));
    assert_eq!(slots[45].item, item_stack(90, 1));
}

#[test]
fn crafting_result_merges_incompatible_default_remainder_into_external_offhand() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::from([(42, 43)]));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: item_stack(43, 2),
    });
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
        ])
    );
    assert_eq!(
        player_slot_item(&store, VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT),
        item_stack(43, 3)
    );
}

#[test]
fn crafting_result_drops_incompatible_default_remainder_when_inventory_is_full() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::from([(42, 43)]));
    store.set_default_item_max_stack_sizes(BTreeMap::from([(43, 1), (99, 1)]));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: item_stack(99, 1),
    });
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 2);
    for slot in 10..46 {
        items[slot] = item_stack(99, 1);
    }
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

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
        ])
    );
    assert_eq!(store.inventory().cursor_item, item_stack(90, 1));
    assert_eq!(
        player_slot_item(&store, VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT),
        item_stack(99, 1)
    );
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

    let slots = &store.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, item_stack(91, 2));
    assert_eq!(slots[1].item, item_stack(42, 2));
}

#[test]
fn apply_local_crafter_quick_move_moves_grid_to_player_backwards() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTER_3X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafter"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[2] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 2,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (2, ProtocolHashedStack::Empty),
            (44, hashed_item_stack(42, 3))
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, 2),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(open_container_slot_item(&store, 44), item_stack(42, 3));
}

#[test]
fn apply_local_crafter_quick_move_skips_disabled_grid_slots() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTER_3X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafter"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[9] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 7,
        id: 0,
        value: 1,
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 9,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (1, hashed_item_stack(42, 3)),
            (9, ProtocolHashedStack::Empty)
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, 0),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(open_container_slot_item(&store, 1), item_stack(42, 3));
    assert_eq!(
        open_container_slot_item(&store, 9),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_crafter_result_slot_requires_server_authority() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTER_3X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafter"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[CRAFTER_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    for input in [
        ProtocolContainerInput::Pickup,
        ProtocolContainerInput::QuickMove,
    ] {
        assert_eq!(
            store.apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: CRAFTER_RESULT_SLOT,
                button_num: 0,
                input,
            }),
            Err(ContainerClickBuildError::UnsupportedLocalClickInput(input))
        );
    }
    let click = store
        .build_container_click_slot(ContainerClickSlotRequest {
            slot_num: CRAFTER_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(click.changed_slots, BTreeMap::new());
    assert_eq!(click.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, CRAFTER_RESULT_SLOT),
        item_stack(90, 1)
    );
}

#[test]
fn apply_local_anvil_input_quick_move_moves_input_slots_to_player_forward() {
    const ANVIL_HOTBAR_START: i16 = 30;

    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: bbb_protocol::RichTextComponent::plain("Anvil"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 39];
    items[ANVIL_INPUT_SLOT as usize] = item_stack(42, 1);
    items[ANVIL_ADDITIONAL_SLOT as usize] = item_stack(43, 2);
    items[ANVIL_RESULT_SLOT as usize] = item_stack(90, 1);
    items[ANVIL_HOTBAR_START as usize] = item_stack(44, 3);
    items[(ANVIL_HOTBAR_START + 1) as usize] = item_stack(45, 2);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let input_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ANVIL_INPUT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        input_move.changed_slots,
        BTreeMap::from([
            (ANVIL_INPUT_SLOT, ProtocolHashedStack::Empty),
            (ANVIL_PLAYER_MAIN_START, hashed_item_stack(42, 1)),
        ])
    );

    let additional_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ANVIL_ADDITIONAL_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        additional_move.changed_slots,
        BTreeMap::from([
            (ANVIL_ADDITIONAL_SLOT, ProtocolHashedStack::Empty),
            (ANVIL_PLAYER_MAIN_START + 1, hashed_item_stack(43, 2)),
        ])
    );

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ANVIL_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::Pickup
        ))
    );
    let result_move_without_cost = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ANVIL_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(result_move_without_cost.changed_slots, BTreeMap::new());
    assert_eq!(
        result_move_without_cost.carried_item,
        ProtocolHashedStack::Empty
    );
    let player_to_input = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ANVIL_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        player_to_input.changed_slots,
        BTreeMap::from([
            (ANVIL_INPUT_SLOT, hashed_item_stack(44, 3)),
            (ANVIL_HOTBAR_START, ProtocolHashedStack::Empty),
        ])
    );

    let player_to_additional = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ANVIL_HOTBAR_START + 1,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        player_to_additional.changed_slots,
        BTreeMap::from([
            (ANVIL_ADDITIONAL_SLOT, hashed_item_stack(45, 2)),
            (ANVIL_HOTBAR_START + 1, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_INPUT_SLOT),
        item_stack(44, 3)
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_ADDITIONAL_SLOT),
        item_stack(45, 2)
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_RESULT_SLOT),
        item_stack(90, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_HOTBAR_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_HOTBAR_START + 1),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_PLAYER_MAIN_START),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_PLAYER_MAIN_START + 1),
        item_stack(43, 2)
    );
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
    renamed_result.component_patch.added = 1;
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
fn apply_local_anvil_result_quick_move_consumes_single_input_when_cost_allows() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: bbb_protocol::RichTextComponent::plain("Anvil"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); ANVIL_TOTAL_SLOT_COUNT as usize];
    items[ANVIL_INPUT_SLOT as usize] = item_stack(42, 1);
    items[ANVIL_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 7,
        id: ANVIL_COST_DATA_ID,
        value: 1,
    });
    store.apply_player_experience(ProtocolPlayerExperience {
        progress: 0.0,
        level: 1,
        total: 0,
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ANVIL_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (ANVIL_INPUT_SLOT, ProtocolHashedStack::Empty),
            (ANVIL_RESULT_SLOT, ProtocolHashedStack::Empty),
            (ANVIL_HOTBAR_END - 1, hashed_item_stack(90, 1)),
        ])
    );
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, ANVIL_INPUT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_ADDITIONAL_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_HOTBAR_END - 1),
        item_stack(90, 1)
    );
}

#[test]
fn apply_local_anvil_result_pickup_consumes_single_input_to_cursor_when_cost_allows() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: bbb_protocol::RichTextComponent::plain("Anvil"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); ANVIL_TOTAL_SLOT_COUNT as usize];
    items[ANVIL_INPUT_SLOT as usize] = item_stack(42, 1);
    items[ANVIL_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 7,
        id: ANVIL_COST_DATA_ID,
        value: 1,
    });
    store.apply_player_experience(ProtocolPlayerExperience {
        progress: 0.0,
        level: 1,
        total: 0,
    });

    let pickup = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ANVIL_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(
        pickup.changed_slots,
        BTreeMap::from([
            (ANVIL_INPUT_SLOT, ProtocolHashedStack::Empty),
            (ANVIL_RESULT_SLOT, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 1));
    assert_eq!(
        open_container_slot_item(&store, ANVIL_INPUT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_ADDITIONAL_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(store.inventory().cursor_item, item_stack(90, 1));
}

#[test]
fn apply_local_anvil_result_quick_move_keeps_low_experience_server_authoritative() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: bbb_protocol::RichTextComponent::plain("Anvil"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); ANVIL_TOTAL_SLOT_COUNT as usize];
    items[ANVIL_INPUT_SLOT as usize] = item_stack(42, 1);
    items[ANVIL_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 7,
        id: ANVIL_COST_DATA_ID,
        value: 2,
    });
    store.apply_player_experience(ProtocolPlayerExperience {
        progress: 0.0,
        level: 1,
        total: 0,
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ANVIL_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(quick_move.changed_slots, BTreeMap::new());
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, ANVIL_INPUT_SLOT),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_RESULT_SLOT),
        item_stack(90, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_HOTBAR_END - 1),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_anvil_result_pickup_keeps_low_experience_server_authoritative() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: bbb_protocol::RichTextComponent::plain("Anvil"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); ANVIL_TOTAL_SLOT_COUNT as usize];
    items[ANVIL_INPUT_SLOT as usize] = item_stack(42, 1);
    items[ANVIL_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 7,
        id: ANVIL_COST_DATA_ID,
        value: 2,
    });
    store.apply_player_experience(ProtocolPlayerExperience {
        progress: 0.0,
        level: 1,
        total: 0,
    });

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ANVIL_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::Pickup
        ))
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_INPUT_SLOT),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_RESULT_SLOT),
        item_stack(90, 1)
    );
    assert_eq!(
        store.inventory().cursor_item,
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_anvil_result_quick_move_keeps_material_input_server_authoritative() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: bbb_protocol::RichTextComponent::plain("Anvil"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); ANVIL_TOTAL_SLOT_COUNT as usize];
    items[ANVIL_INPUT_SLOT as usize] = item_stack(42, 1);
    items[ANVIL_ADDITIONAL_SLOT as usize] = item_stack(43, 1);
    items[ANVIL_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 7,
        id: ANVIL_COST_DATA_ID,
        value: 1,
    });
    store.apply_player_experience(ProtocolPlayerExperience {
        progress: 0.0,
        level: 1,
        total: 0,
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ANVIL_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(quick_move.changed_slots, BTreeMap::new());
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, ANVIL_INPUT_SLOT),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_ADDITIONAL_SLOT),
        item_stack(43, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_RESULT_SLOT),
        item_stack(90, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_HOTBAR_END - 1),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_beacon_quick_move_moves_payment_slot_to_player_reverse() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
        title: bbb_protocol::RichTextComponent::plain("Beacon"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); BEACON_TOTAL_SLOT_COUNT as usize];
    items[BEACON_PAYMENT_SLOT as usize] = item_stack(42, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BEACON_PAYMENT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (BEACON_PAYMENT_SLOT, ProtocolHashedStack::Empty),
            (BEACON_HOTBAR_END - 1, hashed_item_stack(42, 1)),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, BEACON_PAYMENT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, BEACON_HOTBAR_END - 1),
        item_stack(42, 1)
    );
}

#[test]
fn apply_local_beacon_quick_move_routes_single_payment_item_to_payment_slot() {
    let mut store = WorldStore::new();
    apply_item_tags(&mut store, vec![(BEACON_PAYMENT_ITEM_TAG, vec![42])]);
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
        title: bbb_protocol::RichTextComponent::plain("Beacon"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); BEACON_TOTAL_SLOT_COUNT as usize];
    items[BEACON_PLAYER_MAIN_START as usize] = item_stack(42, 1);
    items[(BEACON_PLAYER_MAIN_START + 1) as usize] = item_stack(42, 2);
    items[BEACON_HOTBAR_START as usize] = item_stack(43, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let tagged_stack_with_multiple_items = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BEACON_PLAYER_MAIN_START + 1,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        tagged_stack_with_multiple_items.changed_slots,
        BTreeMap::from([
            (BEACON_PLAYER_MAIN_START + 1, ProtocolHashedStack::Empty),
            (BEACON_HOTBAR_START + 1, hashed_item_stack(42, 2)),
        ])
    );

    let payment_to_slot = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BEACON_PLAYER_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        payment_to_slot.changed_slots,
        BTreeMap::from([
            (BEACON_PAYMENT_SLOT, hashed_item_stack(42, 1)),
            (BEACON_PLAYER_MAIN_START, ProtocolHashedStack::Empty),
        ])
    );

    let hotbar_to_main = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BEACON_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        hotbar_to_main.changed_slots,
        BTreeMap::from([
            (BEACON_PLAYER_MAIN_START, hashed_item_stack(43, 3)),
            (BEACON_HOTBAR_START, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, BEACON_PAYMENT_SLOT),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, BEACON_PLAYER_MAIN_START),
        item_stack(43, 3)
    );
    assert_eq!(
        open_container_slot_item(&store, BEACON_HOTBAR_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, BEACON_HOTBAR_START + 1),
        item_stack(42, 2)
    );
}

#[test]
fn apply_local_beacon_confirm_consumes_payment_and_updates_effect_data() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
        title: bbb_protocol::RichTextComponent::plain("Beacon"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); BEACON_TOTAL_SLOT_COUNT as usize];
    items[BEACON_PAYMENT_SLOT as usize] = item_stack(42, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 15,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert!(store.apply_local_beacon_confirm_effects(4, Some(7)));

    assert_eq!(
        open_container_slot_item(&store, BEACON_PAYMENT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        store.open_container_data_value(BEACON_PRIMARY_EFFECT_DATA_ID),
        Some(5)
    );
    assert_eq!(
        store.open_container_data_value(BEACON_SECONDARY_EFFECT_DATA_ID),
        Some(8)
    );
}

#[test]
fn apply_local_beacon_confirm_clears_missing_secondary_effect_data() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
        title: bbb_protocol::RichTextComponent::plain("Beacon"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); BEACON_TOTAL_SLOT_COUNT as usize];
    items[BEACON_PAYMENT_SLOT as usize] = item_stack(42, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 16,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 7,
        id: BEACON_SECONDARY_EFFECT_DATA_ID,
        value: 8,
    });

    assert!(store.apply_local_beacon_confirm_effects(4, None));

    assert_eq!(
        store.open_container_data_value(BEACON_PRIMARY_EFFECT_DATA_ID),
        Some(5)
    );
    assert_eq!(
        store.open_container_data_value(BEACON_SECONDARY_EFFECT_DATA_ID),
        Some(0)
    );
}

#[test]
fn apply_local_beacon_confirm_requires_beacon_payment() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
        title: bbb_protocol::RichTextComponent::plain("Beacon"),
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 17,
        items: vec![ProtocolItemStackSummary::empty(); BEACON_TOTAL_SLOT_COUNT as usize],
        carried_item: ProtocolItemStackSummary::empty(),
    });
    store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 7,
        id: BEACON_PRIMARY_EFFECT_DATA_ID,
        value: 5,
    });

    assert!(!store.apply_local_beacon_confirm_effects(7, Some(9)));
    assert_eq!(
        store.open_container_data_value(BEACON_PRIMARY_EFFECT_DATA_ID),
        Some(5)
    );
    assert_eq!(
        open_container_slot_item(&store, BEACON_PAYMENT_SLOT),
        ProtocolItemStackSummary::empty()
    );

    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 8,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: bbb_protocol::RichTextComponent::plain("Anvil"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); ANVIL_TOTAL_SLOT_COUNT as usize];
    items[BEACON_PAYMENT_SLOT as usize] = item_stack(42, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 8,
        state_id: 18,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert!(!store.apply_local_beacon_confirm_effects(4, Some(7)));
    assert_eq!(
        open_container_slot_item(&store, BEACON_PAYMENT_SLOT),
        item_stack(42, 1)
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
    direct.component_patch.added = 1;
    direct
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID);
    direct.component_patch.provides_banner_pattern_registry_ids = vec![2, 0];
    open_loom_with_pattern_stack(&mut store, direct);
    assert_eq!(store.loom_selectable_banner_pattern_ids(), Some(vec![2, 0]));

    let mut named = item_stack(44, 1);
    named.component_patch.added = 1;
    named
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID);
    named.component_patch.provides_banner_pattern_tag = Some("example:item_patterns".to_string());
    open_loom_with_pattern_stack(&mut store, named);
    assert_eq!(store.loom_selectable_banner_pattern_ids(), Some(vec![2, 0]));

    let mut unknown = item_stack(44, 1);
    unknown.component_patch.added = 1;
    unknown
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID);
    unknown.component_patch.provides_banner_pattern_registry_ids = vec![9];
    open_loom_with_pattern_stack(&mut store, unknown);
    assert_eq!(store.loom_selectable_banner_pattern_ids(), Some(Vec::new()));

    let mut conflicting = item_stack(44, 1);
    conflicting.component_patch.added = 1;
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
    store.set_registry_dependent_item_component_initializers(BTreeMap::from([(
        44,
        vec![
            RegistryDependentItemComponentInitializer::ProvidesBannerPatterns {
                tag: "example:item_patterns".to_string(),
            },
        ],
    )]));
    store.rebuild_item_component_defaults().unwrap();

    open_loom_with_pattern_stack(&mut store, item_stack(44, 1));
    assert_eq!(store.loom_selectable_banner_pattern_ids(), Some(vec![2, 0]));

    let mut explicit = item_stack(44, 1);
    explicit.component_patch.added = 1;
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
    banner.component_patch.added = 1;
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
    dye.component_patch.added = 1;
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
fn local_container_button_click_rejects_loom_screen_at_six_pattern_limit() {
    let mut store = WorldStore::new();
    install_loom_banner_pattern_registry(&mut store);
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
        title: bbb_protocol::RichTextComponent::plain("Loom"),
    });
    let mut banner = item_stack(42, 1);
    banner.component_patch.added = 1;
    banner
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID);
    banner.component_patch.banner_pattern_layers =
        vec![bbb_protocol::packets::BannerPatternLayerSummary::default(); 6];
    let mut dye = item_stack(43, 1);
    dye.component_patch.added = 1;
    dye.component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_DYE_TYPE_ID);
    dye.component_patch.dye_color_id = Some(VANILLA_DYE_COLOR_RED_ID);
    let mut items = vec![ProtocolItemStackSummary::empty(); LOOM_TOTAL_SLOT_COUNT as usize];
    items[LOOM_BANNER_SLOT as usize] = banner;
    items[LOOM_DYE_SLOT as usize] = dye;
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert_eq!(store.apply_local_container_button_click(0), None);
    assert!(store
        .take_pending_local_interaction_sound_actions()
        .is_empty());
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
fn apply_local_loom_result_quick_move_consumes_single_banner_and_dye() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
        title: bbb_protocol::RichTextComponent::plain("Loom"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); LOOM_TOTAL_SLOT_COUNT as usize];
    items[LOOM_BANNER_SLOT as usize] = item_stack(42, 1);
    items[LOOM_DYE_SLOT as usize] = item_stack(43, 1);
    items[LOOM_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: LOOM_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (LOOM_BANNER_SLOT, ProtocolHashedStack::Empty),
            (LOOM_DYE_SLOT, ProtocolHashedStack::Empty),
            (LOOM_RESULT_SLOT, ProtocolHashedStack::Empty),
            (LOOM_HOTBAR_END - 1, hashed_item_stack(90, 1)),
        ])
    );
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, LOOM_BANNER_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_DYE_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_HOTBAR_END - 1),
        item_stack(90, 1)
    );
}

#[test]
fn apply_local_loom_result_pickup_consumes_single_banner_and_dye_to_cursor() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
        title: bbb_protocol::RichTextComponent::plain("Loom"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); LOOM_TOTAL_SLOT_COUNT as usize];
    items[LOOM_BANNER_SLOT as usize] = item_stack(42, 1);
    items[LOOM_DYE_SLOT as usize] = item_stack(43, 1);
    items[LOOM_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let pickup = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: LOOM_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(
        pickup.changed_slots,
        BTreeMap::from([
            (LOOM_BANNER_SLOT, ProtocolHashedStack::Empty),
            (LOOM_DYE_SLOT, ProtocolHashedStack::Empty),
            (LOOM_RESULT_SLOT, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 1));
    assert_eq!(
        open_container_slot_item(&store, LOOM_BANNER_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_DYE_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(store.inventory().cursor_item, item_stack(90, 1));
}

#[test]
fn apply_local_loom_result_quick_move_keeps_stacked_inputs_server_authoritative() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
        title: bbb_protocol::RichTextComponent::plain("Loom"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); LOOM_TOTAL_SLOT_COUNT as usize];
    items[LOOM_BANNER_SLOT as usize] = item_stack(42, 2);
    items[LOOM_DYE_SLOT as usize] = item_stack(43, 1);
    items[LOOM_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: LOOM_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::Pickup
        ))
    );

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: LOOM_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(quick_move.changed_slots, BTreeMap::new());
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, LOOM_BANNER_SLOT),
        item_stack(42, 2)
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_DYE_SLOT),
        item_stack(43, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_RESULT_SLOT),
        item_stack(90, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_HOTBAR_END - 1),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_loom_result_pickup_keeps_pattern_item_result_server_authoritative() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
        title: bbb_protocol::RichTextComponent::plain("Loom"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); LOOM_TOTAL_SLOT_COUNT as usize];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(43, 1);
    items[2] = item_stack(44, 1);
    items[LOOM_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: LOOM_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::Pickup
        ))
    );

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: LOOM_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(quick_move.changed_slots, BTreeMap::new());
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(open_container_slot_item(&store, 0), item_stack(42, 1));
    assert_eq!(open_container_slot_item(&store, 1), item_stack(43, 1));
    assert_eq!(open_container_slot_item(&store, 2), item_stack(44, 1));
    assert_eq!(
        open_container_slot_item(&store, LOOM_RESULT_SLOT),
        item_stack(90, 1)
    );
}

#[test]
fn apply_local_loom_quick_move_moves_input_slots_to_player_forward() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
        title: bbb_protocol::RichTextComponent::plain("Loom"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); LOOM_TOTAL_SLOT_COUNT as usize];
    items[LOOM_BANNER_SLOT as usize] = item_stack(42, 3);
    items[LOOM_DYE_SLOT as usize] = item_stack(43, 2);
    items[LOOM_PATTERN_SLOT as usize] = item_stack(44, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let banner_to_player = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: LOOM_BANNER_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        banner_to_player.changed_slots,
        BTreeMap::from([
            (LOOM_BANNER_SLOT, ProtocolHashedStack::Empty),
            (LOOM_PLAYER_MAIN_START, hashed_item_stack(42, 3)),
        ])
    );

    let dye_to_player = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: LOOM_DYE_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        dye_to_player.changed_slots,
        BTreeMap::from([
            (LOOM_DYE_SLOT, ProtocolHashedStack::Empty),
            (LOOM_PLAYER_MAIN_START + 1, hashed_item_stack(43, 2)),
        ])
    );

    let pattern_to_player = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: LOOM_PATTERN_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        pattern_to_player.changed_slots,
        BTreeMap::from([
            (LOOM_PATTERN_SLOT, ProtocolHashedStack::Empty),
            (LOOM_PLAYER_MAIN_START + 2, hashed_item_stack(44, 1)),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_BANNER_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_DYE_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_PATTERN_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_PLAYER_MAIN_START),
        item_stack(42, 3)
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_PLAYER_MAIN_START + 1),
        item_stack(43, 2)
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_PLAYER_MAIN_START + 2),
        item_stack(44, 1)
    );
}

#[test]
fn apply_local_loom_quick_move_routes_tagged_items_to_input_slots() {
    let mut store = WorldStore::new();
    apply_item_tags(
        &mut store,
        vec![
            (LOOM_BANNER_ITEM_TAG, vec![42]),
            (LOOM_DYE_ITEM_TAG, vec![43]),
            (LOOM_PATTERN_ITEM_TAG, vec![44]),
        ],
    );
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
        title: bbb_protocol::RichTextComponent::plain("Loom"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); LOOM_TOTAL_SLOT_COUNT as usize];
    items[LOOM_PLAYER_MAIN_START as usize] = item_stack(42, 1);
    items[(LOOM_PLAYER_MAIN_START + 1) as usize] = item_stack(43, 2);
    items[(LOOM_PLAYER_MAIN_START + 2) as usize] = item_stack(44, 1);
    items[LOOM_HOTBAR_START as usize] = item_stack(45, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let banner_to_input = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: LOOM_PLAYER_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        banner_to_input.changed_slots,
        BTreeMap::from([
            (LOOM_BANNER_SLOT, hashed_item_stack(42, 1)),
            (LOOM_PLAYER_MAIN_START, ProtocolHashedStack::Empty),
        ])
    );

    let dye_to_input = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: LOOM_PLAYER_MAIN_START + 1,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        dye_to_input.changed_slots,
        BTreeMap::from([
            (LOOM_DYE_SLOT, hashed_item_stack(43, 2)),
            (LOOM_PLAYER_MAIN_START + 1, ProtocolHashedStack::Empty),
        ])
    );

    let pattern_to_input = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: LOOM_PLAYER_MAIN_START + 2,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        pattern_to_input.changed_slots,
        BTreeMap::from([
            (LOOM_PATTERN_SLOT, hashed_item_stack(44, 1)),
            (LOOM_PLAYER_MAIN_START + 2, ProtocolHashedStack::Empty),
        ])
    );

    let hotbar_to_main = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: LOOM_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        hotbar_to_main.changed_slots,
        BTreeMap::from([
            (LOOM_PLAYER_MAIN_START, hashed_item_stack(45, 3)),
            (LOOM_HOTBAR_START, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_BANNER_SLOT),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_DYE_SLOT),
        item_stack(43, 2)
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_PATTERN_SLOT),
        item_stack(44, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_PLAYER_MAIN_START),
        item_stack(45, 3)
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_HOTBAR_START),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_merchant_quick_move_moves_non_result_slots() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); MERCHANT_TOTAL_SLOT_COUNT as usize];
    items[MERCHANT_PAYMENT_SLOT_1 as usize] = item_stack(42, 3);
    items[MERCHANT_PAYMENT_SLOT_2 as usize] = item_stack(43, 1);
    items[MERCHANT_RESULT_SLOT as usize] = item_stack(90, 1);
    items[MERCHANT_PLAYER_MAIN_START as usize] = item_stack(44, 2);
    items[MERCHANT_HOTBAR_START as usize] = item_stack(45, 4);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::Pickup
        ))
    );
    let result_move_without_offer = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(result_move_without_offer.changed_slots, BTreeMap::new());
    assert_eq!(
        result_move_without_offer.carried_item,
        ProtocolHashedStack::Empty
    );

    let payment_1_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_PAYMENT_SLOT_1,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        payment_1_move.changed_slots,
        BTreeMap::from([
            (MERCHANT_PAYMENT_SLOT_1, ProtocolHashedStack::Empty),
            (MERCHANT_PLAYER_MAIN_START + 1, hashed_item_stack(42, 3)),
        ])
    );

    let payment_2_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_PAYMENT_SLOT_2,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        payment_2_move.changed_slots,
        BTreeMap::from([
            (MERCHANT_PAYMENT_SLOT_2, ProtocolHashedStack::Empty),
            (MERCHANT_PLAYER_MAIN_START + 2, hashed_item_stack(43, 1)),
        ])
    );

    let main_to_hotbar = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_PLAYER_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        main_to_hotbar.changed_slots,
        BTreeMap::from([
            (MERCHANT_PLAYER_MAIN_START, ProtocolHashedStack::Empty),
            (MERCHANT_HOTBAR_START + 1, hashed_item_stack(44, 2)),
        ])
    );

    let hotbar_to_main = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        hotbar_to_main.changed_slots,
        BTreeMap::from([
            (MERCHANT_PLAYER_MAIN_START, hashed_item_stack(45, 4)),
            (MERCHANT_HOTBAR_START, ProtocolHashedStack::Empty),
        ])
    );

    let click = store
        .build_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(click.changed_slots, BTreeMap::new());
    assert_eq!(click.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_1),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_2),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_RESULT_SLOT),
        item_stack(90, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PLAYER_MAIN_START),
        item_stack(45, 4)
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PLAYER_MAIN_START + 1),
        item_stack(42, 3)
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PLAYER_MAIN_START + 2),
        item_stack(43, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_HOTBAR_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_HOTBAR_START + 1),
        item_stack(44, 2)
    );
}

#[test]
fn apply_local_merchant_result_quick_move_consumes_exact_selected_offer_payments() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); MERCHANT_TOTAL_SLOT_COUNT as usize];
    items[MERCHANT_PAYMENT_SLOT_1 as usize] = item_stack(42, 7);
    items[MERCHANT_PAYMENT_SLOT_2 as usize] = item_stack(43, 2);
    items[MERCHANT_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert!(store.apply_merchant_offers(ProtocolMerchantOffers {
        container_id: 7,
        offers: vec![ProtocolMerchantOffer {
            buy_a: item_cost(42, 5),
            sell: item_stack(90, 1),
            buy_b: Some(item_cost(43, 2)),
            is_out_of_stock: false,
            uses: 1,
            max_uses: 12,
            xp: 8,
            special_price_diff: -1,
            price_multiplier: 0.1,
            demand: 6,
        }],
        villager_level: 3,
        villager_xp: 120,
        show_progress: true,
        can_restock: false,
    }));

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (MERCHANT_PAYMENT_SLOT_1, ProtocolHashedStack::Empty),
            (MERCHANT_PAYMENT_SLOT_2, ProtocolHashedStack::Empty),
            (MERCHANT_RESULT_SLOT, ProtocolHashedStack::Empty),
            (MERCHANT_HOTBAR_END - 1, hashed_item_stack(90, 1)),
        ])
    );
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_1),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_2),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_HOTBAR_END - 1),
        item_stack(90, 1)
    );
    let offers = store
        .inventory()
        .open_container
        .as_ref()
        .and_then(|container| container.merchant_offers.as_ref())
        .unwrap();
    assert_eq!(offers.offers[0].uses, 2);
    assert!(!offers.offers[0].is_out_of_stock);
}

#[test]
fn apply_local_merchant_result_pickup_places_result_on_cursor_and_keeps_payable_remainder() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); MERCHANT_TOTAL_SLOT_COUNT as usize];
    items[MERCHANT_PAYMENT_SLOT_1 as usize] = item_stack(42, 7);
    items[MERCHANT_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 15,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert!(store.apply_merchant_offers(ProtocolMerchantOffers {
        container_id: 7,
        offers: vec![ProtocolMerchantOffer {
            buy_a: item_cost(42, 3),
            sell: item_stack(90, 1),
            buy_b: None,
            is_out_of_stock: false,
            uses: 1,
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

    let pickup = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(
        pickup.changed_slots,
        BTreeMap::from([(MERCHANT_PAYMENT_SLOT_1, hashed_item_stack(42, 4))])
    );
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 1));
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_1),
        item_stack(42, 4)
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_RESULT_SLOT),
        item_stack(90, 1)
    );
    assert_eq!(store.inventory().cursor_item, item_stack(90, 1));
    let offers = store
        .inventory()
        .open_container
        .as_ref()
        .and_then(|container| container.merchant_offers.as_ref())
        .unwrap();
    assert_eq!(offers.offers[0].uses, 2);
}

#[test]
fn apply_local_merchant_result_secondary_pickup_requires_server_authority() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); MERCHANT_TOTAL_SLOT_COUNT as usize];
    items[MERCHANT_PAYMENT_SLOT_1 as usize] = item_stack(42, 3);
    items[MERCHANT_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 16,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert!(store.apply_merchant_offers(ProtocolMerchantOffers {
        container_id: 7,
        offers: vec![ProtocolMerchantOffer {
            buy_a: item_cost(42, 3),
            sell: item_stack(90, 1),
            buy_b: None,
            is_out_of_stock: false,
            uses: 1,
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

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_RESULT_SLOT,
            button_num: 1,
            input: ProtocolContainerInput::Pickup,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::Pickup
        ))
    );
}

#[test]
fn apply_local_merchant_result_quick_move_consumes_overfilled_payment_remainder() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); MERCHANT_TOTAL_SLOT_COUNT as usize];
    items[MERCHANT_PAYMENT_SLOT_1 as usize] = item_stack(42, 4);
    items[MERCHANT_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 15,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert!(store.apply_merchant_offers(ProtocolMerchantOffers {
        container_id: 7,
        offers: vec![ProtocolMerchantOffer {
            buy_a: item_cost(42, 3),
            sell: item_stack(90, 1),
            buy_b: None,
            is_out_of_stock: false,
            uses: 1,
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

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (MERCHANT_PAYMENT_SLOT_1, hashed_item_stack(42, 1)),
            (MERCHANT_RESULT_SLOT, ProtocolHashedStack::Empty),
            (MERCHANT_HOTBAR_END - 1, hashed_item_stack(90, 1)),
        ])
    );
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_1),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_merchant_result_quick_move_repopulates_result_when_remainder_still_pays() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); MERCHANT_TOTAL_SLOT_COUNT as usize];
    items[MERCHANT_PAYMENT_SLOT_1 as usize] = item_stack(42, 7);
    items[MERCHANT_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 16,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert!(store.apply_merchant_offers(ProtocolMerchantOffers {
        container_id: 7,
        offers: vec![ProtocolMerchantOffer {
            buy_a: item_cost(42, 3),
            sell: item_stack(90, 1),
            buy_b: None,
            is_out_of_stock: false,
            uses: 1,
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

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (MERCHANT_PAYMENT_SLOT_1, hashed_item_stack(42, 4)),
            (MERCHANT_HOTBAR_END - 1, hashed_item_stack(90, 1)),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_1),
        item_stack(42, 4)
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_RESULT_SLOT),
        item_stack(90, 1)
    );
    let offers = store
        .inventory()
        .open_container
        .as_ref()
        .and_then(|container| container.merchant_offers.as_ref())
        .unwrap();
    assert_eq!(offers.offers[0].uses, 2);
}

#[test]
fn apply_local_merchant_result_quick_move_consumes_swapped_payment_slots() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); MERCHANT_TOTAL_SLOT_COUNT as usize];
    items[MERCHANT_PAYMENT_SLOT_1 as usize] = item_stack(43, 2);
    items[MERCHANT_PAYMENT_SLOT_2 as usize] = item_stack(42, 4);
    items[MERCHANT_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 17,
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
            uses: 1,
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

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (MERCHANT_PAYMENT_SLOT_1, ProtocolHashedStack::Empty),
            (MERCHANT_PAYMENT_SLOT_2, hashed_item_stack(42, 1)),
            (MERCHANT_RESULT_SLOT, ProtocolHashedStack::Empty),
            (MERCHANT_HOTBAR_END - 1, hashed_item_stack(90, 1)),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_1),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_2),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_merchant_result_quick_move_clears_result_when_offer_runs_out_of_stock() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); MERCHANT_TOTAL_SLOT_COUNT as usize];
    items[MERCHANT_PAYMENT_SLOT_1 as usize] = item_stack(42, 7);
    items[MERCHANT_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 18,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert!(store.apply_merchant_offers(ProtocolMerchantOffers {
        container_id: 7,
        offers: vec![ProtocolMerchantOffer {
            buy_a: item_cost(42, 3),
            sell: item_stack(90, 1),
            buy_b: None,
            is_out_of_stock: false,
            uses: 11,
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

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (MERCHANT_PAYMENT_SLOT_1, hashed_item_stack(42, 4)),
            (MERCHANT_RESULT_SLOT, ProtocolHashedStack::Empty),
            (MERCHANT_HOTBAR_END - 1, hashed_item_stack(90, 1)),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_1),
        item_stack(42, 4)
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    let offers = store
        .inventory()
        .open_container
        .as_ref()
        .and_then(|container| container.merchant_offers.as_ref())
        .unwrap();
    assert_eq!(offers.offers[0].uses, 12);
    assert!(offers.offers[0].is_out_of_stock);
}

#[test]
fn apply_local_merchant_result_quick_move_rejects_component_predicate_cost() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); MERCHANT_TOTAL_SLOT_COUNT as usize];
    items[MERCHANT_PAYMENT_SLOT_1 as usize] = item_stack(42, 3);
    items[MERCHANT_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 16,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    let mut cost = item_cost(42, 3);
    cost.component_predicate.components.added = 1;
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
            uses: 1,
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

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(quick_move.changed_slots, BTreeMap::new());
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_1),
        item_stack(42, 3)
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_RESULT_SLOT),
        item_stack(90, 1)
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
fn set_local_merchant_selected_offer_keeps_component_predicate_cost_server_authoritative() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    let mut matching_payment = item_stack(42, 5);
    matching_payment.component_patch.added = 1;
    matching_payment.component_patch.added_type_ids =
        vec![VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID];
    matching_payment.component_patch.item_model = Some("minecraft:emerald".to_string());
    let mut items = vec![ProtocolItemStackSummary::empty(); MERCHANT_TOTAL_SLOT_COUNT as usize];
    items[MERCHANT_PLAYER_MAIN_START as usize] = matching_payment.clone();
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 15,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    let mut cost = item_cost(42, 1);
    cost.component_predicate.components.added = 1;
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
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PLAYER_MAIN_START),
        matching_payment
    );
    assert!(store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 7,
        state_id: 16,
        slot: MERCHANT_PAYMENT_SLOT_1,
        item: matching_payment.clone(),
    }));
    assert!(store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 7,
        state_id: 17,
        slot: MERCHANT_PLAYER_MAIN_START,
        item: ProtocolItemStackSummary::empty(),
    }));
    assert!(store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 7,
        state_id: 18,
        slot: MERCHANT_RESULT_SLOT,
        item: item_stack(90, 1),
    }));

    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_1),
        matching_payment
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PLAYER_MAIN_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_RESULT_SLOT),
        item_stack(90, 1)
    );
}

#[test]
fn apply_local_enchantment_quick_move_moves_menu_slots_to_player_reverse() {
    const ENCHANTMENT_TOTAL_SLOT_COUNT: usize = 38;

    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ENCHANTMENT_ID,
        title: bbb_protocol::RichTextComponent::plain("Enchanting Table"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); ENCHANTMENT_TOTAL_SLOT_COUNT];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(43, 3);
    items[29] = item_stack(44, 2);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let input_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ENCHANTMENT_INPUT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        input_move.changed_slots,
        BTreeMap::from([
            (ENCHANTMENT_INPUT_SLOT, ProtocolHashedStack::Empty),
            (ENCHANTMENT_HOTBAR_END - 1, hashed_item_stack(42, 1)),
        ])
    );

    let lapis_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ENCHANTMENT_LAPIS_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        lapis_move.changed_slots,
        BTreeMap::from([
            (ENCHANTMENT_LAPIS_SLOT, ProtocolHashedStack::Empty),
            (ENCHANTMENT_HOTBAR_END - 2, hashed_item_stack(43, 3)),
        ])
    );
    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 29,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::QuickMove
        ))
    );
    let click = store
        .build_container_click_slot(ContainerClickSlotRequest {
            slot_num: 29,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(click.changed_slots, BTreeMap::new());
    assert_eq!(click.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, ENCHANTMENT_INPUT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, ENCHANTMENT_LAPIS_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(open_container_slot_item(&store, 29), item_stack(44, 2));
    assert_eq!(
        open_container_slot_item(&store, ENCHANTMENT_HOTBAR_END - 2),
        item_stack(43, 3)
    );
    assert_eq!(
        open_container_slot_item(&store, ENCHANTMENT_HOTBAR_END - 1),
        item_stack(42, 1)
    );
}

#[test]
fn apply_local_enchantment_quick_move_routes_player_items_to_lapis_and_input_slots() {
    const ENCHANTMENT_HOTBAR_START: i16 = 29;

    let mut store = WorldStore::new();
    store.set_enchantment_lapis_lazuli_item_ids(BTreeSet::from([43]));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ENCHANTMENT_ID,
        title: bbb_protocol::RichTextComponent::plain("Enchanting Table"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); ENCHANTMENT_TOTAL_SLOT_COUNT as usize];
    items[ENCHANTMENT_HOTBAR_START as usize] = item_stack(43, 3);
    items[(ENCHANTMENT_HOTBAR_START + 1) as usize] = item_stack(50, 2);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let lapis_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ENCHANTMENT_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        lapis_move.changed_slots,
        BTreeMap::from([
            (ENCHANTMENT_LAPIS_SLOT, hashed_item_stack(43, 3)),
            (ENCHANTMENT_HOTBAR_START, ProtocolHashedStack::Empty),
        ])
    );
    let input_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ENCHANTMENT_HOTBAR_START + 1,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        input_move.changed_slots,
        BTreeMap::from([
            (ENCHANTMENT_INPUT_SLOT, hashed_item_stack(50, 1)),
            (ENCHANTMENT_HOTBAR_START + 1, hashed_item_stack(50, 1)),
        ])
    );
    let occupied_input_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ENCHANTMENT_HOTBAR_START + 1,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(occupied_input_move.changed_slots, BTreeMap::new());
    assert_eq!(
        open_container_slot_item(&store, ENCHANTMENT_LAPIS_SLOT),
        item_stack(43, 3)
    );
    assert_eq!(
        open_container_slot_item(&store, ENCHANTMENT_INPUT_SLOT),
        item_stack(50, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, ENCHANTMENT_HOTBAR_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, ENCHANTMENT_HOTBAR_START + 1),
        item_stack(50, 1)
    );
}

#[test]
fn apply_local_smithing_input_quick_move_moves_input_slots_to_player_forward() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SMITHING_ID,
        title: bbb_protocol::RichTextComponent::plain("Smithing"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); SMITHING_TOTAL_SLOT_COUNT as usize];
    items[SMITHING_TEMPLATE_SLOT as usize] = item_stack(42, 1);
    items[SMITHING_BASE_SLOT as usize] = item_stack(43, 2);
    items[SMITHING_ADDITIONAL_SLOT as usize] = item_stack(44, 3);
    items[SMITHING_RESULT_SLOT as usize] = item_stack(90, 1);
    items[31] = item_stack(45, 4);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let template_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: SMITHING_TEMPLATE_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        template_move.changed_slots,
        BTreeMap::from([
            (SMITHING_TEMPLATE_SLOT, ProtocolHashedStack::Empty),
            (SMITHING_PLAYER_MAIN_START, hashed_item_stack(42, 1)),
        ])
    );

    let base_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: SMITHING_BASE_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        base_move.changed_slots,
        BTreeMap::from([
            (SMITHING_BASE_SLOT, ProtocolHashedStack::Empty),
            (SMITHING_PLAYER_MAIN_START + 1, hashed_item_stack(43, 2)),
        ])
    );

    let additional_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: SMITHING_ADDITIONAL_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        additional_move.changed_slots,
        BTreeMap::from([
            (SMITHING_ADDITIONAL_SLOT, ProtocolHashedStack::Empty),
            (SMITHING_PLAYER_MAIN_START + 2, hashed_item_stack(44, 3)),
        ])
    );

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: SMITHING_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::Pickup
        ))
    );
    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 31,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::QuickMove
        ))
    );
    let click = store
        .build_container_click_slot(ContainerClickSlotRequest {
            slot_num: 31,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(click.changed_slots, BTreeMap::new());
    assert_eq!(click.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, SMITHING_TEMPLATE_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_BASE_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_ADDITIONAL_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_RESULT_SLOT),
        item_stack(90, 1)
    );
    assert_eq!(open_container_slot_item(&store, 31), item_stack(45, 4));
    assert_eq!(
        open_container_slot_item(&store, SMITHING_PLAYER_MAIN_START),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_PLAYER_MAIN_START + 1),
        item_stack(43, 2)
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_PLAYER_MAIN_START + 2),
        item_stack(44, 3)
    );
}

#[test]
fn apply_local_smithing_result_quick_move_consumes_single_inputs() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SMITHING_ID,
        title: bbb_protocol::RichTextComponent::plain("Smithing"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); SMITHING_TOTAL_SLOT_COUNT as usize];
    items[SMITHING_TEMPLATE_SLOT as usize] = item_stack(42, 1);
    items[SMITHING_BASE_SLOT as usize] = item_stack(43, 1);
    items[SMITHING_ADDITIONAL_SLOT as usize] = item_stack(44, 1);
    items[SMITHING_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: SMITHING_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (SMITHING_TEMPLATE_SLOT, ProtocolHashedStack::Empty),
            (SMITHING_BASE_SLOT, ProtocolHashedStack::Empty),
            (SMITHING_ADDITIONAL_SLOT, ProtocolHashedStack::Empty),
            (SMITHING_RESULT_SLOT, ProtocolHashedStack::Empty),
            (SMITHING_HOTBAR_END - 1, hashed_item_stack(90, 1)),
        ])
    );
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, SMITHING_TEMPLATE_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_BASE_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_ADDITIONAL_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_HOTBAR_END - 1),
        item_stack(90, 1)
    );
}

#[test]
fn apply_local_smithing_result_pickup_consumes_single_inputs_to_cursor() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SMITHING_ID,
        title: bbb_protocol::RichTextComponent::plain("Smithing"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); SMITHING_TOTAL_SLOT_COUNT as usize];
    items[SMITHING_TEMPLATE_SLOT as usize] = item_stack(42, 1);
    items[SMITHING_BASE_SLOT as usize] = item_stack(43, 1);
    items[SMITHING_ADDITIONAL_SLOT as usize] = item_stack(44, 1);
    items[SMITHING_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let pickup = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: SMITHING_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(
        pickup.changed_slots,
        BTreeMap::from([
            (SMITHING_TEMPLATE_SLOT, ProtocolHashedStack::Empty),
            (SMITHING_BASE_SLOT, ProtocolHashedStack::Empty),
            (SMITHING_ADDITIONAL_SLOT, ProtocolHashedStack::Empty),
            (SMITHING_RESULT_SLOT, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 1));
    assert_eq!(
        open_container_slot_item(&store, SMITHING_TEMPLATE_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_BASE_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_ADDITIONAL_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(store.inventory().cursor_item, item_stack(90, 1));
}

#[test]
fn apply_local_smithing_result_quick_move_keeps_stacked_inputs_server_authoritative() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SMITHING_ID,
        title: bbb_protocol::RichTextComponent::plain("Smithing"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); SMITHING_TOTAL_SLOT_COUNT as usize];
    items[SMITHING_TEMPLATE_SLOT as usize] = item_stack(42, 2);
    items[SMITHING_BASE_SLOT as usize] = item_stack(43, 1);
    items[SMITHING_ADDITIONAL_SLOT as usize] = item_stack(44, 1);
    items[SMITHING_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: SMITHING_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(quick_move.changed_slots, BTreeMap::new());
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, SMITHING_TEMPLATE_SLOT),
        item_stack(42, 2)
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_BASE_SLOT),
        item_stack(43, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_ADDITIONAL_SLOT),
        item_stack(44, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_RESULT_SLOT),
        item_stack(90, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_HOTBAR_END - 1),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_smithing_result_pickup_keeps_stacked_inputs_server_authoritative() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SMITHING_ID,
        title: bbb_protocol::RichTextComponent::plain("Smithing"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); SMITHING_TOTAL_SLOT_COUNT as usize];
    items[SMITHING_TEMPLATE_SLOT as usize] = item_stack(42, 2);
    items[SMITHING_BASE_SLOT as usize] = item_stack(43, 1);
    items[SMITHING_ADDITIONAL_SLOT as usize] = item_stack(44, 1);
    items[SMITHING_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: SMITHING_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::Pickup
        ))
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_TEMPLATE_SLOT),
        item_stack(42, 2)
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_RESULT_SLOT),
        item_stack(90, 1)
    );
    assert_eq!(
        store.inventory().cursor_item,
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_smithing_quick_move_routes_property_set_items_to_inputs_and_player_ranges() {
    let mut store = WorldStore::new();
    store.apply_update_recipes(update_recipes(vec![
        (SMITHING_TEMPLATE_PROPERTY_SET, vec![42]),
        (SMITHING_BASE_PROPERTY_SET, vec![43]),
        (SMITHING_ADDITION_PROPERTY_SET, vec![44]),
    ]));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SMITHING_ID,
        title: bbb_protocol::RichTextComponent::plain("Smithing"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); SMITHING_TOTAL_SLOT_COUNT as usize];
    items[SMITHING_PLAYER_MAIN_START as usize] = item_stack(50, 2);
    items[SMITHING_HOTBAR_START as usize] = item_stack(42, 1);
    items[(SMITHING_HOTBAR_START + 1) as usize] = item_stack(43, 2);
    items[(SMITHING_HOTBAR_START + 2) as usize] = item_stack(44, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let template_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: SMITHING_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        template_move.changed_slots,
        BTreeMap::from([
            (SMITHING_TEMPLATE_SLOT, hashed_item_stack(42, 1)),
            (SMITHING_HOTBAR_START, ProtocolHashedStack::Empty),
        ])
    );

    let base_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: SMITHING_HOTBAR_START + 1,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        base_move.changed_slots,
        BTreeMap::from([
            (SMITHING_BASE_SLOT, hashed_item_stack(43, 2)),
            (SMITHING_HOTBAR_START + 1, ProtocolHashedStack::Empty),
        ])
    );

    let additional_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: SMITHING_HOTBAR_START + 2,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        additional_move.changed_slots,
        BTreeMap::from([
            (SMITHING_ADDITIONAL_SLOT, hashed_item_stack(44, 3)),
            (SMITHING_HOTBAR_START + 2, ProtocolHashedStack::Empty),
        ])
    );

    let range_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: SMITHING_PLAYER_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        range_move.changed_slots,
        BTreeMap::from([
            (SMITHING_PLAYER_MAIN_START, ProtocolHashedStack::Empty),
            (SMITHING_HOTBAR_START, hashed_item_stack(50, 2)),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_TEMPLATE_SLOT),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_BASE_SLOT),
        item_stack(43, 2)
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_ADDITIONAL_SLOT),
        item_stack(44, 3)
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_HOTBAR_START),
        item_stack(50, 2)
    );
}

#[test]
fn apply_local_cartography_table_result_pickup_consumes_single_inputs_to_cursor() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
        title: bbb_protocol::RichTextComponent::plain("Cartography Table"),
    });
    let mut items =
        vec![ProtocolItemStackSummary::empty(); CARTOGRAPHY_TABLE_TOTAL_SLOT_COUNT as usize];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(43, 1);
    items[CARTOGRAPHY_TABLE_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let pickup = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: CARTOGRAPHY_TABLE_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(
        pickup.changed_slots,
        BTreeMap::from([
            (CARTOGRAPHY_TABLE_MAP_SLOT, ProtocolHashedStack::Empty),
            (
                CARTOGRAPHY_TABLE_ADDITIONAL_SLOT,
                ProtocolHashedStack::Empty
            ),
            (CARTOGRAPHY_TABLE_RESULT_SLOT, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 1));
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_MAP_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_ADDITIONAL_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(store.inventory().cursor_item, item_stack(90, 1));
}

#[test]
fn apply_local_cartography_table_result_quick_move_predicts_single_input_consumption() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
        title: bbb_protocol::RichTextComponent::plain("Cartography Table"),
    });
    let mut items =
        vec![ProtocolItemStackSummary::empty(); CARTOGRAPHY_TABLE_TOTAL_SLOT_COUNT as usize];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(43, 1);
    items[CARTOGRAPHY_TABLE_RESULT_SLOT as usize] = item_stack(90, 1);
    items[30] = item_stack(44, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let result_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: CARTOGRAPHY_TABLE_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        result_move.changed_slots,
        BTreeMap::from([
            (CARTOGRAPHY_TABLE_MAP_SLOT, ProtocolHashedStack::Empty),
            (
                CARTOGRAPHY_TABLE_ADDITIONAL_SLOT,
                ProtocolHashedStack::Empty
            ),
            (CARTOGRAPHY_TABLE_RESULT_SLOT, ProtocolHashedStack::Empty),
            (CARTOGRAPHY_TABLE_HOTBAR_END - 1, hashed_item_stack(90, 1)),
        ])
    );
    assert_eq!(result_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 30,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::QuickMove
        ))
    );
    let click = store
        .build_container_click_slot(ContainerClickSlotRequest {
            slot_num: 30,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(click.changed_slots, BTreeMap::new());
    assert_eq!(click.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_MAP_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_ADDITIONAL_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(open_container_slot_item(&store, 30), item_stack(44, 3));
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_HOTBAR_END - 1),
        item_stack(90, 1)
    );
}

#[test]
fn apply_local_cartography_table_result_quick_move_keeps_stacked_inputs_server_authoritative() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
        title: bbb_protocol::RichTextComponent::plain("Cartography Table"),
    });
    let mut items =
        vec![ProtocolItemStackSummary::empty(); CARTOGRAPHY_TABLE_TOTAL_SLOT_COUNT as usize];
    items[CARTOGRAPHY_TABLE_MAP_SLOT as usize] = item_stack(42, 2);
    items[CARTOGRAPHY_TABLE_ADDITIONAL_SLOT as usize] = item_stack(43, 1);
    items[CARTOGRAPHY_TABLE_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: CARTOGRAPHY_TABLE_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::Pickup
        ))
    );
    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: CARTOGRAPHY_TABLE_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(quick_move.changed_slots, BTreeMap::new());
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_MAP_SLOT),
        item_stack(42, 2)
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_ADDITIONAL_SLOT),
        item_stack(43, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_RESULT_SLOT),
        item_stack(90, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_HOTBAR_END - 1),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_cartography_table_quick_move_moves_input_slots_to_player_forward() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
        title: bbb_protocol::RichTextComponent::plain("Cartography Table"),
    });
    let mut items =
        vec![ProtocolItemStackSummary::empty(); CARTOGRAPHY_TABLE_TOTAL_SLOT_COUNT as usize];
    items[CARTOGRAPHY_TABLE_MAP_SLOT as usize] = item_stack(42, 1);
    items[CARTOGRAPHY_TABLE_ADDITIONAL_SLOT as usize] = item_stack(43, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let map_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: CARTOGRAPHY_TABLE_MAP_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        map_move.changed_slots,
        BTreeMap::from([
            (CARTOGRAPHY_TABLE_MAP_SLOT, ProtocolHashedStack::Empty),
            (
                CARTOGRAPHY_TABLE_PLAYER_MAIN_START,
                hashed_item_stack(42, 1)
            ),
        ])
    );
    assert_eq!(map_move.carried_item, ProtocolHashedStack::Empty);

    let additional_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: CARTOGRAPHY_TABLE_ADDITIONAL_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        additional_move.changed_slots,
        BTreeMap::from([
            (
                CARTOGRAPHY_TABLE_ADDITIONAL_SLOT,
                ProtocolHashedStack::Empty
            ),
            (
                CARTOGRAPHY_TABLE_PLAYER_MAIN_START + 1,
                hashed_item_stack(43, 3)
            ),
        ])
    );
    assert_eq!(additional_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_MAP_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_ADDITIONAL_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_PLAYER_MAIN_START),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_PLAYER_MAIN_START + 1),
        item_stack(43, 3)
    );
}

#[test]
fn apply_local_cartography_table_quick_move_routes_additional_and_player_ranges() {
    let mut store = WorldStore::new();
    store.set_cartography_additional_items(BTreeMap::from([
        (43, crate::CartographyAdditionalItemKind::Paper),
        (44, crate::CartographyAdditionalItemKind::EmptyMap),
        (45, crate::CartographyAdditionalItemKind::GlassPane),
    ]));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
        title: bbb_protocol::RichTextComponent::plain("Cartography Table"),
    });
    let mut items =
        vec![ProtocolItemStackSummary::empty(); CARTOGRAPHY_TABLE_TOTAL_SLOT_COUNT as usize];
    items[CARTOGRAPHY_TABLE_PLAYER_MAIN_START as usize] = item_stack(43, 2);
    items[(CARTOGRAPHY_TABLE_PLAYER_MAIN_START + 1) as usize] = item_stack(50, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let additional_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: CARTOGRAPHY_TABLE_PLAYER_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        additional_move.changed_slots,
        BTreeMap::from([
            (CARTOGRAPHY_TABLE_ADDITIONAL_SLOT, hashed_item_stack(43, 2)),
            (
                CARTOGRAPHY_TABLE_PLAYER_MAIN_START,
                ProtocolHashedStack::Empty
            ),
        ])
    );

    let main_to_hotbar = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: CARTOGRAPHY_TABLE_PLAYER_MAIN_START + 1,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        main_to_hotbar.changed_slots,
        BTreeMap::from([
            (
                CARTOGRAPHY_TABLE_PLAYER_MAIN_START + 1,
                ProtocolHashedStack::Empty
            ),
            (CARTOGRAPHY_TABLE_HOTBAR_START, hashed_item_stack(50, 3)),
        ])
    );

    let hotbar_to_main = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: CARTOGRAPHY_TABLE_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        hotbar_to_main.changed_slots,
        BTreeMap::from([
            (
                CARTOGRAPHY_TABLE_PLAYER_MAIN_START,
                hashed_item_stack(50, 3)
            ),
            (CARTOGRAPHY_TABLE_HOTBAR_START, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_ADDITIONAL_SLOT),
        item_stack(43, 2)
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_PLAYER_MAIN_START),
        item_stack(50, 3)
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_HOTBAR_START),
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
fn apply_local_cartography_table_map_id_with_removed_component_quick_move_routes_to_map_slot() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
        title: bbb_protocol::RichTextComponent::plain("Cartography Table"),
    });
    let mut items =
        vec![ProtocolItemStackSummary::empty(); CARTOGRAPHY_TABLE_TOTAL_SLOT_COUNT as usize];
    let mut map_stack = map_id_item_stack(42, 1, 7);
    map_stack
        .component_patch
        .removed_type_ids
        .push(VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID);
    items[CARTOGRAPHY_TABLE_PLAYER_MAIN_START as usize] = map_stack.clone();
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
                hashed_map_id_removed_component_item_stack(
                    42,
                    1,
                    7,
                    VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID
                )
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
        map_stack
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_PLAYER_MAIN_START),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn cartography_table_map_id_with_use_effects_component_predicts() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
        title: bbb_protocol::RichTextComponent::plain("Cartography Table"),
    });
    let mut items =
        vec![ProtocolItemStackSummary::empty(); CARTOGRAPHY_TABLE_TOTAL_SLOT_COUNT as usize];
    let mut map_stack = map_id_item_stack(42, 1, 7);
    map_stack.component_patch.added = 2;
    map_stack
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID);
    map_stack.component_patch.use_effects = Some(ProtocolUseEffectsSummary {
        can_sprint: true,
        interact_vibrations: true,
        speed_multiplier: 1.0,
    });
    items[CARTOGRAPHY_TABLE_PLAYER_MAIN_START as usize] = map_stack.clone();
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let click = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: CARTOGRAPHY_TABLE_PLAYER_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        click.changed_slots,
        BTreeMap::from([
            (
                CARTOGRAPHY_TABLE_MAP_SLOT,
                ProtocolHashedStack::Item(ProtocolHashedItemStack {
                    item_id: 42,
                    count: 1,
                    components: ProtocolHashedComponentPatch {
                        added_components: BTreeMap::from([
                            (VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID, 1_133_294_169,),
                            (
                                VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID,
                                hash_ops_crc32c_int(7),
                            ),
                        ]),
                        removed_components: BTreeSet::new(),
                    },
                }),
            ),
            (
                CARTOGRAPHY_TABLE_PLAYER_MAIN_START,
                ProtocolHashedStack::Empty,
            ),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_MAP_SLOT),
        map_stack
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_PLAYER_MAIN_START),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_stonecutter_quick_move_moves_input_slot_to_player_forward() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); STONECUTTER_TOTAL_SLOT_COUNT as usize];
    items[STONECUTTER_INPUT_SLOT as usize] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: STONECUTTER_INPUT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (STONECUTTER_INPUT_SLOT, ProtocolHashedStack::Empty),
            (STONECUTTER_PLAYER_MAIN_START, hashed_item_stack(42, 3)),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, STONECUTTER_INPUT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, STONECUTTER_PLAYER_MAIN_START),
        item_stack(42, 3)
    );
}

#[test]
fn apply_local_stonecutter_quick_move_moves_player_main_and_hotbar_ranges() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); STONECUTTER_TOTAL_SLOT_COUNT as usize];
    items[STONECUTTER_PLAYER_MAIN_START as usize] = item_stack(42, 3);
    items[STONECUTTER_HOTBAR_START as usize] = item_stack(43, 4);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let main_to_hotbar = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: STONECUTTER_PLAYER_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        main_to_hotbar.changed_slots,
        BTreeMap::from([
            (STONECUTTER_PLAYER_MAIN_START, ProtocolHashedStack::Empty),
            (STONECUTTER_HOTBAR_START + 1, hashed_item_stack(42, 3)),
        ])
    );

    let hotbar_to_main = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: STONECUTTER_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        hotbar_to_main.changed_slots,
        BTreeMap::from([
            (STONECUTTER_PLAYER_MAIN_START, hashed_item_stack(43, 4)),
            (STONECUTTER_HOTBAR_START, ProtocolHashedStack::Empty),
        ])
    );

    assert_eq!(
        open_container_slot_item(&store, STONECUTTER_PLAYER_MAIN_START),
        item_stack(43, 4)
    );
    assert_eq!(
        open_container_slot_item(&store, STONECUTTER_HOTBAR_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, STONECUTTER_HOTBAR_START + 1),
        item_stack(42, 3)
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
fn apply_local_stonecutter_result_pickup_consumes_single_input_to_cursor() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); STONECUTTER_TOTAL_SLOT_COUNT as usize];
    items[STONECUTTER_INPUT_SLOT as usize] = item_stack(42, 1);
    items[STONECUTTER_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 15,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let pickup = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: STONECUTTER_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(
        pickup.changed_slots,
        BTreeMap::from([
            (STONECUTTER_INPUT_SLOT, ProtocolHashedStack::Empty),
            (STONECUTTER_RESULT_SLOT, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 1));
    assert_eq!(
        open_container_slot_item(&store, STONECUTTER_INPUT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, STONECUTTER_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(store.inventory().cursor_item, item_stack(90, 1));
}

#[test]
fn apply_local_stonecutter_result_quick_move_predicts_single_input_consumption() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); STONECUTTER_TOTAL_SLOT_COUNT as usize];
    items[STONECUTTER_INPUT_SLOT as usize] = item_stack(42, 1);
    items[STONECUTTER_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 15,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: STONECUTTER_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (STONECUTTER_INPUT_SLOT, ProtocolHashedStack::Empty),
            (STONECUTTER_RESULT_SLOT, ProtocolHashedStack::Empty),
            (STONECUTTER_HOTBAR_END - 1, hashed_item_stack(90, 1)),
        ])
    );
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, STONECUTTER_INPUT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, STONECUTTER_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, STONECUTTER_HOTBAR_END - 1),
        item_stack(90, 1)
    );
}

#[test]
fn apply_local_stonecutter_result_pickup_keeps_remaining_input_server_authoritative() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); STONECUTTER_TOTAL_SLOT_COUNT as usize];
    items[STONECUTTER_INPUT_SLOT as usize] = item_stack(42, 2);
    items[STONECUTTER_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 16,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: STONECUTTER_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::Pickup
        ))
    );
    assert_eq!(
        open_container_slot_item(&store, STONECUTTER_INPUT_SLOT),
        item_stack(42, 2)
    );
    assert_eq!(
        open_container_slot_item(&store, STONECUTTER_RESULT_SLOT),
        item_stack(90, 1)
    );
    assert_eq!(
        store.inventory().cursor_item,
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_stonecutter_result_quick_move_keeps_blocked_transfer_server_authoritative() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); STONECUTTER_TOTAL_SLOT_COUNT as usize];
    items[STONECUTTER_INPUT_SLOT as usize] = item_stack(42, 1);
    items[STONECUTTER_RESULT_SLOT as usize] = item_stack(90, 1);
    for slot in STONECUTTER_PLAYER_MAIN_START..STONECUTTER_HOTBAR_END {
        items[slot as usize] = item_stack(91, 64);
    }
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 15,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: STONECUTTER_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(quick_move.changed_slots, BTreeMap::new());
    assert_eq!(
        open_container_slot_item(&store, STONECUTTER_INPUT_SLOT),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, STONECUTTER_RESULT_SLOT),
        item_stack(90, 1)
    );
}

#[test]
fn apply_local_brewing_stand_quick_move_moves_brewing_slots_to_player_reverse() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BREWING_STAND_ID,
        title: bbb_protocol::RichTextComponent::plain("Brewing Stand"),
    });
    let mut items =
        vec![ProtocolItemStackSummary::empty(); BREWING_STAND_TOTAL_SLOT_COUNT as usize];
    items[BREWING_STAND_BOTTLE_SLOT_START as usize] = item_stack(42, 1);
    items[BREWING_STAND_INGREDIENT_SLOT as usize] = item_stack(43, 2);
    items[BREWING_STAND_FUEL_SLOT as usize] = item_stack(44, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let bottle_to_player = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BREWING_STAND_BOTTLE_SLOT_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        bottle_to_player.changed_slots,
        BTreeMap::from([
            (BREWING_STAND_BOTTLE_SLOT_START, ProtocolHashedStack::Empty),
            (BREWING_STAND_HOTBAR_END - 1, hashed_item_stack(42, 1)),
        ])
    );

    let ingredient_to_player = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BREWING_STAND_INGREDIENT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        ingredient_to_player.changed_slots,
        BTreeMap::from([
            (BREWING_STAND_INGREDIENT_SLOT, ProtocolHashedStack::Empty),
            (BREWING_STAND_HOTBAR_END - 2, hashed_item_stack(43, 2)),
        ])
    );

    let fuel_to_player = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BREWING_STAND_FUEL_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        fuel_to_player.changed_slots,
        BTreeMap::from([
            (BREWING_STAND_FUEL_SLOT, ProtocolHashedStack::Empty),
            (BREWING_STAND_HOTBAR_END - 3, hashed_item_stack(44, 3)),
        ])
    );
}

#[test]
fn apply_local_brewing_stand_quick_move_routes_player_items_to_brewing_slots() {
    let mut store = WorldStore::new();
    store.set_default_item_max_stack_sizes(BTreeMap::from([(42, 64)]));
    store.set_brewing_potion_item_ids(BTreeSet::from([42]));
    store.set_brewing_ingredient_item_ids(BTreeSet::from([43]));
    apply_item_tags(&mut store, vec![(BREWING_STAND_FUEL_ITEM_TAG, vec![44])]);
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BREWING_STAND_ID,
        title: bbb_protocol::RichTextComponent::plain("Brewing Stand"),
    });
    let mut items =
        vec![ProtocolItemStackSummary::empty(); BREWING_STAND_TOTAL_SLOT_COUNT as usize];
    items[BREWING_STAND_PLAYER_MAIN_START as usize] = item_stack(42, 3);
    items[(BREWING_STAND_PLAYER_MAIN_START + 1) as usize] = item_stack(43, 2);
    items[(BREWING_STAND_PLAYER_MAIN_START + 2) as usize] = item_stack(44, 5);
    items[BREWING_STAND_HOTBAR_START as usize] = item_stack(45, 4);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let potion_to_bottle_slot = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BREWING_STAND_PLAYER_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        potion_to_bottle_slot.changed_slots,
        BTreeMap::from([
            (BREWING_STAND_BOTTLE_SLOT_START, hashed_item_stack(42, 1)),
            (BREWING_STAND_PLAYER_MAIN_START, hashed_item_stack(42, 2)),
        ])
    );

    let ingredient_to_slot = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BREWING_STAND_PLAYER_MAIN_START + 1,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        ingredient_to_slot.changed_slots,
        BTreeMap::from([
            (BREWING_STAND_INGREDIENT_SLOT, hashed_item_stack(43, 2)),
            (
                BREWING_STAND_PLAYER_MAIN_START + 1,
                ProtocolHashedStack::Empty
            ),
        ])
    );

    let fuel_to_slot = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BREWING_STAND_PLAYER_MAIN_START + 2,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        fuel_to_slot.changed_slots,
        BTreeMap::from([
            (BREWING_STAND_FUEL_SLOT, hashed_item_stack(44, 5)),
            (
                BREWING_STAND_PLAYER_MAIN_START + 2,
                ProtocolHashedStack::Empty
            ),
        ])
    );

    let hotbar_to_main = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BREWING_STAND_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        hotbar_to_main.changed_slots,
        BTreeMap::from([
            (
                BREWING_STAND_PLAYER_MAIN_START + 1,
                hashed_item_stack(45, 4)
            ),
            (BREWING_STAND_HOTBAR_START, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, BREWING_STAND_BOTTLE_SLOT_START),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, BREWING_STAND_PLAYER_MAIN_START),
        item_stack(42, 2)
    );
    assert_eq!(
        open_container_slot_item(&store, BREWING_STAND_INGREDIENT_SLOT),
        item_stack(43, 2)
    );
    assert_eq!(
        open_container_slot_item(&store, BREWING_STAND_FUEL_SLOT),
        item_stack(44, 5)
    );
    assert_eq!(
        open_container_slot_item(&store, BREWING_STAND_HOTBAR_START),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_grindstone_input_quick_move_moves_inputs_to_player_forward() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
        title: bbb_protocol::RichTextComponent::plain("Grindstone"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); GRINDSTONE_TOTAL_SLOT_COUNT as usize];
    items[GRINDSTONE_INPUT_SLOT as usize] = item_stack(42, 1);
    items[GRINDSTONE_ADDITIONAL_SLOT as usize] = item_stack(43, 2);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let input_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: GRINDSTONE_INPUT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        input_move.changed_slots,
        BTreeMap::from([
            (GRINDSTONE_INPUT_SLOT, ProtocolHashedStack::Empty),
            (GRINDSTONE_PLAYER_MAIN_START, hashed_item_stack(42, 1)),
        ])
    );

    let additional_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: GRINDSTONE_ADDITIONAL_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        additional_move.changed_slots,
        BTreeMap::from([
            (GRINDSTONE_ADDITIONAL_SLOT, ProtocolHashedStack::Empty),
            (GRINDSTONE_PLAYER_MAIN_START + 1, hashed_item_stack(43, 2)),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_INPUT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_ADDITIONAL_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_PLAYER_MAIN_START),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_PLAYER_MAIN_START + 1),
        item_stack(43, 2)
    );
}

#[test]
fn apply_local_grindstone_player_to_input_quick_move_requires_server_authority() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
        title: bbb_protocol::RichTextComponent::plain("Grindstone"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); GRINDSTONE_TOTAL_SLOT_COUNT as usize];
    items[GRINDSTONE_PLAYER_MAIN_START as usize] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let request = ContainerClickSlotRequest {
        slot_num: GRINDSTONE_PLAYER_MAIN_START,
        button_num: 0,
        input: ProtocolContainerInput::QuickMove,
    };
    assert_eq!(
        store.apply_local_container_click_slot(request),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::QuickMove
        ))
    );
    let click = store.build_container_click_slot(request).unwrap();
    assert_eq!(click.changed_slots, BTreeMap::new());
    assert_eq!(click.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_INPUT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_PLAYER_MAIN_START),
        item_stack(42, 3)
    );
}

#[test]
fn apply_local_grindstone_quick_move_routes_default_damageable_items_to_inputs() {
    let mut store = WorldStore::new();
    store.set_default_damageable_item_ids(BTreeSet::from([42, 43]));
    store.set_default_item_max_stack_sizes(BTreeMap::from([(42, 1), (43, 1)]));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
        title: bbb_protocol::RichTextComponent::plain("Grindstone"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); GRINDSTONE_TOTAL_SLOT_COUNT as usize];
    items[GRINDSTONE_PLAYER_MAIN_START as usize] = item_stack(42, 1);
    items[(GRINDSTONE_PLAYER_MAIN_START + 1) as usize] = item_stack(43, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let first_input = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: GRINDSTONE_PLAYER_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        first_input.changed_slots,
        BTreeMap::from([
            (GRINDSTONE_INPUT_SLOT, hashed_item_stack(42, 1)),
            (GRINDSTONE_PLAYER_MAIN_START, ProtocolHashedStack::Empty),
        ])
    );

    let second_input = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: GRINDSTONE_PLAYER_MAIN_START + 1,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        second_input.changed_slots,
        BTreeMap::from([
            (GRINDSTONE_ADDITIONAL_SLOT, hashed_item_stack(43, 1)),
            (GRINDSTONE_PLAYER_MAIN_START + 1, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_INPUT_SLOT),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_ADDITIONAL_SLOT),
        item_stack(43, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_PLAYER_MAIN_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_PLAYER_MAIN_START + 1),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_grindstone_quick_move_moves_player_ranges_when_inputs_full() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
        title: bbb_protocol::RichTextComponent::plain("Grindstone"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); GRINDSTONE_TOTAL_SLOT_COUNT as usize];
    items[GRINDSTONE_INPUT_SLOT as usize] = item_stack(10, 1);
    items[GRINDSTONE_ADDITIONAL_SLOT as usize] = item_stack(11, 1);
    items[GRINDSTONE_PLAYER_MAIN_START as usize] = item_stack(42, 3);
    items[GRINDSTONE_HOTBAR_START as usize] = item_stack(43, 4);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let main_to_hotbar = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: GRINDSTONE_PLAYER_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        main_to_hotbar.changed_slots,
        BTreeMap::from([
            (GRINDSTONE_PLAYER_MAIN_START, ProtocolHashedStack::Empty),
            (GRINDSTONE_HOTBAR_START + 1, hashed_item_stack(42, 3)),
        ])
    );

    let hotbar_to_main = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: GRINDSTONE_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        hotbar_to_main.changed_slots,
        BTreeMap::from([
            (GRINDSTONE_PLAYER_MAIN_START, hashed_item_stack(43, 4)),
            (GRINDSTONE_HOTBAR_START, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_PLAYER_MAIN_START),
        item_stack(43, 4)
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_HOTBAR_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_HOTBAR_START + 1),
        item_stack(42, 3)
    );
}

#[test]
fn apply_local_grindstone_result_pickup_consumes_inputs_to_cursor() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
        title: bbb_protocol::RichTextComponent::plain("Grindstone"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); GRINDSTONE_TOTAL_SLOT_COUNT as usize];
    items[GRINDSTONE_INPUT_SLOT as usize] = item_stack(42, 1);
    items[GRINDSTONE_ADDITIONAL_SLOT as usize] = item_stack(43, 1);
    items[GRINDSTONE_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 16,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let pickup = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: GRINDSTONE_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(
        pickup.changed_slots,
        BTreeMap::from([
            (GRINDSTONE_INPUT_SLOT, ProtocolHashedStack::Empty),
            (GRINDSTONE_ADDITIONAL_SLOT, ProtocolHashedStack::Empty),
            (GRINDSTONE_RESULT_SLOT, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 1));
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_INPUT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_ADDITIONAL_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(store.inventory().cursor_item, item_stack(90, 1));
}

#[test]
fn apply_local_grindstone_result_secondary_pickup_requires_server_authority() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
        title: bbb_protocol::RichTextComponent::plain("Grindstone"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); GRINDSTONE_TOTAL_SLOT_COUNT as usize];
    items[GRINDSTONE_INPUT_SLOT as usize] = item_stack(42, 1);
    items[GRINDSTONE_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 16,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: GRINDSTONE_RESULT_SLOT,
            button_num: 1,
            input: ProtocolContainerInput::Pickup,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::Pickup
        ))
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_INPUT_SLOT),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_RESULT_SLOT),
        item_stack(90, 1)
    );
}

#[test]
fn apply_local_grindstone_result_quick_move_predicts() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
        title: bbb_protocol::RichTextComponent::plain("Grindstone"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); GRINDSTONE_TOTAL_SLOT_COUNT as usize];
    items[GRINDSTONE_INPUT_SLOT as usize] = item_stack(42, 1);
    items[GRINDSTONE_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 16,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: GRINDSTONE_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (GRINDSTONE_INPUT_SLOT, ProtocolHashedStack::Empty),
            (GRINDSTONE_RESULT_SLOT, ProtocolHashedStack::Empty),
            (GRINDSTONE_HOTBAR_END - 1, hashed_item_stack(90, 1)),
        ])
    );
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_INPUT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_HOTBAR_END - 1),
        item_stack(90, 1)
    );
}

#[test]
fn apply_local_grindstone_result_quick_move_keeps_blocked_transfer_server_authoritative() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
        title: bbb_protocol::RichTextComponent::plain("Grindstone"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); GRINDSTONE_TOTAL_SLOT_COUNT as usize];
    items[GRINDSTONE_INPUT_SLOT as usize] = item_stack(42, 1);
    items[GRINDSTONE_RESULT_SLOT as usize] = item_stack(90, 1);
    for slot in GRINDSTONE_PLAYER_MAIN_START..GRINDSTONE_HOTBAR_END {
        items[slot as usize] = item_stack(91, 64);
    }
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 16,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: GRINDSTONE_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(quick_move.changed_slots, BTreeMap::new());
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_INPUT_SLOT),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_RESULT_SLOT),
        item_stack(90, 1)
    );
}

#[test]
fn apply_local_hopper_quick_move_moves_hopper_to_player_reverse() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_HOPPER_ID,
        title: bbb_protocol::RichTextComponent::plain("Hopper"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 41];
    items[0] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(quick_move.container_id, 7);
    assert_eq!(quick_move.state_id, 12);
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (0, ProtocolHashedStack::Empty),
            (40, hashed_item_stack(42, 3))
        ])
    );
    let slots = &store.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[40].item, item_stack(42, 3));
}

#[test]
fn apply_local_hopper_quick_move_moves_player_to_hopper_forward() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_HOPPER_ID,
        title: bbb_protocol::RichTextComponent::plain("Hopper"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 41];
    items[5] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 5,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(quick_move.container_id, 7);
    assert_eq!(quick_move.state_id, 13);
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (0, hashed_item_stack(42, 3)),
            (5, ProtocolHashedStack::Empty)
        ])
    );
    let slots = &store.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, item_stack(42, 3));
    assert_eq!(slots[5].item, ProtocolItemStackSummary::empty());
}

#[test]
fn apply_local_shulker_box_quick_move_moves_shulker_to_player_reverse() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SHULKER_BOX_ID,
        title: bbb_protocol::RichTextComponent::plain("Shulker Box"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 63];
    items[0] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(quick_move.container_id, 7);
    assert_eq!(quick_move.state_id, 12);
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (0, ProtocolHashedStack::Empty),
            (62, hashed_item_stack(42, 3))
        ])
    );
    let slots = &store.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[62].item, item_stack(42, 3));
}

#[test]
fn apply_local_shulker_box_quick_move_moves_player_to_shulker_forward() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SHULKER_BOX_ID,
        title: bbb_protocol::RichTextComponent::plain("Shulker Box"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 63];
    items[27] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 27,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(quick_move.container_id, 7);
    assert_eq!(quick_move.state_id, 13);
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (0, hashed_item_stack(42, 3)),
            (27, ProtocolHashedStack::Empty)
        ])
    );
    let slots = &store.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, item_stack(42, 3));
    assert_eq!(slots[27].item, ProtocolItemStackSummary::empty());
}

#[test]
fn apply_local_furnace_quick_move_moves_result_to_player_reverse() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_FURNACE_ID,
        title: bbb_protocol::RichTextComponent::plain("Furnace"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 39];
    items[2] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 2,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(quick_move.container_id, 7);
    assert_eq!(quick_move.state_id, 12);
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (2, ProtocolHashedStack::Empty),
            (38, hashed_item_stack(42, 3)),
        ])
    );
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, 2),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(open_container_slot_item(&store, 38), item_stack(42, 3));
}

#[test]
fn apply_local_furnace_quick_move_routes_input_and_fuel_to_furnace_slots() {
    let mut store = WorldStore::new();
    store.apply_update_recipes(update_recipes(vec![("minecraft:furnace_input", vec![42])]));
    store.set_furnace_fuel_item_ids(BTreeSet::from([43]));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_FURNACE_ID,
        title: bbb_protocol::RichTextComponent::plain("Furnace"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 39];
    items[3] = item_stack(42, 3);
    items[30] = item_stack(43, 2);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let input_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 3,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    let fuel_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 30,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        input_move.changed_slots,
        BTreeMap::from([
            (0, hashed_item_stack(42, 3)),
            (3, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(
        fuel_move.changed_slots,
        BTreeMap::from([
            (1, hashed_item_stack(43, 2)),
            (30, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(open_container_slot_item(&store, 0), item_stack(42, 3));
    assert_eq!(open_container_slot_item(&store, 1), item_stack(43, 2));
    assert_eq!(
        open_container_slot_item(&store, 3),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, 30),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_furnace_quick_move_uses_menu_specific_input_property_set() {
    let mut store = WorldStore::new();
    store.apply_update_recipes(update_recipes(vec![("minecraft:furnace_input", vec![42])]));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BLAST_FURNACE_ID,
        title: bbb_protocol::RichTextComponent::plain("Blast Furnace"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 39];
    items[3] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let fallback_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 3,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        fallback_move.changed_slots,
        BTreeMap::from([
            (3, ProtocolHashedStack::Empty),
            (30, hashed_item_stack(42, 3)),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, 0),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(open_container_slot_item(&store, 30), item_stack(42, 3));

    store.apply_update_recipes(update_recipes(vec![(
        "minecraft:blast_furnace_input",
        vec![42],
    )]));
    store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 7,
        state_id: 13,
        slot: 30,
        item: item_stack(42, 3),
    });

    let blast_input_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 30,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        blast_input_move.changed_slots,
        BTreeMap::from([
            (0, hashed_item_stack(42, 3)),
            (30, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(open_container_slot_item(&store, 0), item_stack(42, 3));
    assert_eq!(
        open_container_slot_item(&store, 30),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_furnace_quick_move_prioritizes_input_over_fuel() {
    let mut store = WorldStore::new();
    store.apply_update_recipes(update_recipes(vec![("minecraft:smoker_input", vec![42])]));
    store.set_furnace_fuel_item_ids(BTreeSet::from([42]));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SMOKER_ID,
        title: bbb_protocol::RichTextComponent::plain("Smoker"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 39];
    items[30] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 30,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (0, hashed_item_stack(42, 3)),
            (30, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(open_container_slot_item(&store, 0), item_stack(42, 3));
    assert_eq!(
        open_container_slot_item(&store, 1),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_furnace_quick_move_moves_input_and_fuel_slots_to_player_forward() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_FURNACE_ID,
        title: bbb_protocol::RichTextComponent::plain("Furnace"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 39];
    items[0] = item_stack(42, 3);
    items[1] = item_stack(43, 2);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let input_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    let fuel_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 1,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        input_move.changed_slots,
        BTreeMap::from([
            (0, ProtocolHashedStack::Empty),
            (3, hashed_item_stack(42, 3)),
        ])
    );
    assert_eq!(
        fuel_move.changed_slots,
        BTreeMap::from([
            (1, ProtocolHashedStack::Empty),
            (4, hashed_item_stack(43, 2)),
        ])
    );
    assert_eq!(open_container_slot_item(&store, 3), item_stack(42, 3));
    assert_eq!(open_container_slot_item(&store, 4), item_stack(43, 2));
}

#[test]
fn apply_local_container_quick_move_rejects_non_inventory_menu() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: i32::try_from(VANILLA_MENU_TYPES.len()).unwrap(),
        title: bbb_protocol::RichTextComponent::plain("Unsupported"),
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items: vec![item_stack(42, 3); 9],
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert_eq!(
        store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 0,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap_err(),
        ContainerClickBuildError::UnsupportedLocalClickInput(ProtocolContainerInput::QuickMove)
    );
    assert_eq!(
        store.inventory().open_container.as_ref().unwrap().slots[0].item,
        item_stack(42, 3)
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
fn apply_local_container_throw_drops_one_from_inventory_slot() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 3),
    });
    assert!(store.open_local_inventory());

    let throw = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::Throw,
        })
        .unwrap();

    assert_eq!(throw.container_id, INVENTORY_MENU_CONTAINER_ID);
    assert_eq!(throw.input, ProtocolContainerInput::Throw);
    assert_eq!(
        throw.changed_slots,
        BTreeMap::from([(INVENTORY_MENU_HOTBAR_START, hashed_item_stack(42, 2))])
    );
    assert_eq!(throw.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
        item_stack(42, 2)
    );
    assert_eq!(player_slot_item(&store, 0), item_stack(42, 2));
}

#[test]
fn apply_local_container_throw_drops_full_stack_and_requires_empty_cursor() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 3),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 1,
        item: item_stack(43, 4),
    });
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(99, 1),
    });
    assert!(store.open_local_inventory());

    let blocked = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_HOTBAR_START,
            button_num: 1,
            input: ProtocolContainerInput::Throw,
        })
        .unwrap();
    assert!(blocked.changed_slots.is_empty());
    assert_eq!(blocked.carried_item, hashed_item_stack(99, 1));
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
        item_stack(42, 3)
    );

    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: ProtocolItemStackSummary::empty(),
    });
    let drop_stack = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_HOTBAR_START + 1,
            button_num: 1,
            input: ProtocolContainerInput::Throw,
        })
        .unwrap();

    assert_eq!(
        drop_stack.changed_slots,
        BTreeMap::from([(INVENTORY_MENU_HOTBAR_START + 1, ProtocolHashedStack::Empty)])
    );
    assert_eq!(drop_stack.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START + 1),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        player_slot_item(&store, 1),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_container_throw_predicts_generic_menu() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X2_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 54];
    items[0] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let throw = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::Throw,
        })
        .unwrap();

    assert_eq!(
        throw.changed_slots,
        BTreeMap::from([(0, hashed_item_stack(42, 2))])
    );
    assert_eq!(throw.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(open_container_slot_item(&store, 0), item_stack(42, 2));
}

#[test]
fn apply_local_anvil_throw_predicts_inputs_and_keeps_result_actions_authoritative() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: bbb_protocol::RichTextComponent::plain("Anvil"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); ANVIL_TOTAL_SLOT_COUNT as usize];
    items[ANVIL_INPUT_SLOT as usize] = item_stack(42, 3);
    items[ANVIL_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let throw_input = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ANVIL_INPUT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Throw,
        })
        .unwrap();
    assert_eq!(
        throw_input.changed_slots,
        BTreeMap::from([(ANVIL_INPUT_SLOT, hashed_item_stack(42, 2))])
    );

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ANVIL_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Throw,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::Throw
        ))
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_RESULT_SLOT),
        item_stack(90, 1)
    );
}

#[test]
fn apply_local_container_swap_exchanges_hovered_slot_with_hotbar_button() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 3),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 9,
        item: item_stack(43, 2),
    });
    assert!(store.open_local_inventory());

    let swap = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::Swap,
        })
        .unwrap();

    assert_eq!(swap.container_id, INVENTORY_MENU_CONTAINER_ID);
    assert_eq!(swap.input, ProtocolContainerInput::Swap);
    assert_eq!(
        swap.changed_slots,
        BTreeMap::from([
            (INVENTORY_MENU_MAIN_START, hashed_item_stack(42, 3)),
            (INVENTORY_MENU_HOTBAR_START, hashed_item_stack(43, 2)),
        ])
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
        item_stack(42, 3)
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
        item_stack(43, 2)
    );
    assert_eq!(player_slot_item(&store, 9), item_stack(42, 3));
    assert_eq!(player_slot_item(&store, 0), item_stack(43, 2));
}

#[test]
fn apply_local_container_swap_moves_hovered_slot_to_empty_offhand() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 9,
        item: item_stack(44, 5),
    });
    assert!(store.open_local_inventory());

    let swap = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_MAIN_START,
            button_num: 40,
            input: ProtocolContainerInput::Swap,
        })
        .unwrap();

    assert_eq!(
        swap.changed_slots,
        BTreeMap::from([
            (INVENTORY_MENU_MAIN_START, ProtocolHashedStack::Empty),
            (INVENTORY_MENU_OFFHAND_SLOT, hashed_item_stack(44, 5)),
        ])
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_OFFHAND_SLOT),
        item_stack(44, 5)
    );
    assert_eq!(
        player_slot_item(&store, 9),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        player_slot_item(&store, VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT),
        item_stack(44, 5)
    );
}

#[test]
fn apply_local_container_swap_splits_source_when_target_slot_has_lower_limit() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(45, 3),
    });
    assert!(store.open_local_inventory());

    let swap = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 5,
            button_num: 0,
            input: ProtocolContainerInput::Swap,
        })
        .unwrap();

    assert_eq!(
        swap.changed_slots,
        BTreeMap::from([
            (5, hashed_item_stack(45, 1)),
            (INVENTORY_MENU_HOTBAR_START, hashed_item_stack(45, 2)),
        ])
    );
    assert_eq!(inventory_menu_slot_item(&store, 5), item_stack(45, 1));
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
        item_stack(45, 2)
    );
    assert_eq!(player_slot_item(&store, 39), item_stack(45, 1));
    assert_eq!(player_slot_item(&store, 0), item_stack(45, 2));
}

#[test]
fn apply_local_container_swap_requires_empty_cursor_and_predicts_generic_menu() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 3),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 9,
        item: item_stack(43, 2),
    });
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(99, 1),
    });
    assert!(store.open_local_inventory());

    let blocked = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::Swap,
        })
        .unwrap();
    assert!(blocked.changed_slots.is_empty());
    assert_eq!(blocked.carried_item, hashed_item_stack(99, 1));
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
        item_stack(43, 2)
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
        item_stack(42, 3)
    );

    store.close_local_container(INVENTORY_MENU_CONTAINER_ID);
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X2_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 54];
    items[0] = item_stack(42, 3);
    items[45] = item_stack(43, 2);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let swap = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::Swap,
        })
        .unwrap();

    assert_eq!(
        swap.changed_slots,
        BTreeMap::from([
            (0, hashed_item_stack(43, 2)),
            (45, hashed_item_stack(42, 3)),
        ])
    );
    assert_eq!(open_container_slot_item(&store, 0), item_stack(43, 2));
    assert_eq!(open_container_slot_item(&store, 45), item_stack(42, 3));
    assert_eq!(player_slot_item(&store, 0), item_stack(42, 3));
}

#[test]
fn apply_local_open_container_swap_uses_external_offhand_without_hashing_it() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: item_stack(44, 2),
    });
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X2_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 54];
    items[0] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 4,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let swap = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT as i8,
            input: ProtocolContainerInput::Swap,
        })
        .unwrap();

    assert_eq!(
        swap.changed_slots,
        BTreeMap::from([(0, hashed_item_stack(44, 2))])
    );
    assert_eq!(open_container_slot_item(&store, 0), item_stack(44, 2));
    assert_eq!(
        player_slot_item(&store, VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT),
        item_stack(42, 3)
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_OFFHAND_SLOT),
        item_stack(42, 3)
    );
}

#[test]
fn apply_local_open_container_offhand_swap_returns_displaced_overflow_to_inventory() {
    let mut store = WorldStore::new();
    apply_item_tags(&mut store, vec![(BEACON_PAYMENT_ITEM_TAG, vec![90])]);
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: item_stack(90, 3),
    });
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
        title: bbb_protocol::RichTextComponent::plain("Beacon"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); BEACON_TOTAL_SLOT_COUNT as usize];
    items[BEACON_PAYMENT_SLOT as usize] = item_stack(42, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 5,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let swap = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BEACON_PAYMENT_SLOT,
            button_num: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT as i8,
            input: ProtocolContainerInput::Swap,
        })
        .unwrap();

    assert_eq!(
        swap.changed_slots,
        BTreeMap::from([
            (BEACON_PAYMENT_SLOT, hashed_item_stack(90, 1)),
            (BEACON_HOTBAR_START, hashed_item_stack(42, 1)),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, BEACON_PAYMENT_SLOT),
        item_stack(90, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, BEACON_HOTBAR_START),
        item_stack(42, 1)
    );
    assert_eq!(
        player_slot_item(&store, VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT),
        item_stack(90, 2)
    );
}

#[test]
fn apply_local_beacon_swap_enforces_payment_tag_and_single_item_limit() {
    let mut store = WorldStore::new();
    apply_item_tags(&mut store, vec![(BEACON_PAYMENT_ITEM_TAG, vec![42])]);
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
        title: bbb_protocol::RichTextComponent::plain("Beacon"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); BEACON_TOTAL_SLOT_COUNT as usize];
    let hotbar_slot = BEACON_HOTBAR_START;
    items[hotbar_slot as usize] = item_stack(43, 2);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let rejected = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BEACON_PAYMENT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Swap,
        })
        .unwrap();
    assert_eq!(rejected.changed_slots, BTreeMap::new());

    store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 7,
        state_id: 4,
        slot: hotbar_slot,
        item: item_stack(42, 3),
    });
    let accepted = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BEACON_PAYMENT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Swap,
        })
        .unwrap();

    assert_eq!(
        accepted.changed_slots,
        BTreeMap::from([
            (BEACON_PAYMENT_SLOT, hashed_item_stack(42, 1)),
            (hotbar_slot, hashed_item_stack(42, 2)),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, BEACON_PAYMENT_SLOT),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, hotbar_slot),
        item_stack(42, 2)
    );
}

#[test]
fn apply_local_container_clone_copies_slot_stack_to_cursor_at_max_count() {
    let mut store = WorldStore::new();
    store.set_default_item_max_stack_sizes(BTreeMap::from([(42, 16)]));
    apply_player_instabuild(&mut store, true);
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X2_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items: vec![item_stack(42, 3)],
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let clone = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 2,
            input: ProtocolContainerInput::Clone,
        })
        .unwrap();

    assert_eq!(clone.container_id, 7);
    assert_eq!(clone.state_id, 3);
    assert_eq!(clone.input, ProtocolContainerInput::Clone);
    assert!(clone.changed_slots.is_empty());
    assert_eq!(clone.carried_item, hashed_item_stack(42, 16));
    assert_eq!(open_container_slot_item(&store, 0), item_stack(42, 3));
    assert_eq!(store.inventory().cursor_item, item_stack(42, 16));
}

#[test]
fn apply_local_container_clone_does_not_apply_without_instabuild() {
    let mut store = WorldStore::new();
    store.set_default_item_max_stack_sizes(BTreeMap::from([(42, 16)]));
    apply_player_instabuild(&mut store, false);
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X2_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items: vec![item_stack(42, 3)],
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let blocked = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 2,
            input: ProtocolContainerInput::Clone,
        })
        .unwrap();

    assert!(blocked.changed_slots.is_empty());
    assert_eq!(blocked.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(open_container_slot_item(&store, 0), item_stack(42, 3));
    assert_eq!(
        store.inventory().cursor_item,
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_container_clone_does_not_apply_with_non_empty_cursor() {
    let mut store = WorldStore::new();
    store.set_default_item_max_stack_sizes(BTreeMap::from([(42, 16)]));
    apply_player_instabuild(&mut store, true);
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X2_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items: vec![item_stack(42, 3)],
        carried_item: item_stack(99, 1),
    });

    let blocked = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 2,
            input: ProtocolContainerInput::Clone,
        })
        .unwrap();

    assert!(blocked.changed_slots.is_empty());
    assert_eq!(blocked.carried_item, hashed_item_stack(99, 1));
    assert_eq!(open_container_slot_item(&store, 0), item_stack(42, 3));
    assert_eq!(store.inventory().cursor_item, item_stack(99, 1));
}

fn mount_visibility_for_entity(
    entity_type_id: i32,
    data_values: Vec<ProtocolEntityDataValue>,
) -> Option<MountEquipmentSlotVisibility> {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(42, entity_type_id));
    if !data_values.is_empty() {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 42,
            values: data_values,
        }));
    }
    store.apply_mount_screen_open(ProtocolMountScreenOpen {
        container_id: 7,
        inventory_columns: 3,
        entity_id: 42,
    });
    store.open_mount_equipment_slot_visibility()
}

fn protocol_add_entity_with_type(id: i32, entity_type_id: i32) -> ProtocolAddEntity {
    ProtocolAddEntity {
        id,
        uuid: Uuid::from_u128(id as u128),
        entity_type_id,
        position: ProtocolVec3d {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        },
        delta_movement: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    }
}

fn protocol_byte_data(data_id: u8, value: i8) -> ProtocolEntityDataValue {
    ProtocolEntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        value: EntityDataValueKind::Byte(value),
    }
}

fn protocol_bool_data(data_id: u8, value: bool) -> ProtocolEntityDataValue {
    ProtocolEntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        value: EntityDataValueKind::Boolean(value),
    }
}

fn item_stack(item_id: i32, count: i32) -> ProtocolItemStackSummary {
    ProtocolItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    }
}

fn install_loom_banner_pattern_registry(store: &mut WorldStore) {
    store.registries.contents.insert(
        BANNER_PATTERN_REGISTRY.to_string(),
        crate::RegistryContentState {
            registry: BANNER_PATTERN_REGISTRY.to_string(),
            packet_count: 1,
            entries: [
                "example:custom",
                "minecraft:stripe",
                "minecraft:flower",
                "minecraft:base",
            ]
            .into_iter()
            .map(crate::RegistryPacketEntry::stub)
            .collect(),
            duplicate_entry_ids: BTreeMap::new(),
        },
    );
    store.registries.tags.insert(
        BANNER_PATTERN_REGISTRY.to_string(),
        RegistryTagState {
            registry: BANNER_PATTERN_REGISTRY.to_string(),
            tags: BTreeMap::from([
                (LOOM_NO_ITEM_REQUIRED_PATTERN_TAG.to_string(), vec![3, 0, 1]),
                ("example:item_patterns".to_string(), vec![2, 0]),
            ]),
        },
    );
}

fn open_loom_with_pattern_stack(store: &mut WorldStore, pattern_stack: ProtocolItemStackSummary) {
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
        title: bbb_protocol::RichTextComponent::plain("Loom"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); LOOM_TOTAL_SLOT_COUNT as usize];
    items[LOOM_BANNER_SLOT as usize] = item_stack(42, 1);
    items[LOOM_DYE_SLOT as usize] = item_stack(43, 1);
    items[LOOM_PATTERN_SLOT as usize] = pattern_stack;
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
}

fn slot_display_empty() -> SlotDisplaySummary {
    SlotDisplaySummary {
        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
        raw_payload: vec![0],
        item_stack: None,
        tag: None,
    }
}

fn slot_display_item(item_id: i32) -> SlotDisplaySummary {
    SlotDisplaySummary {
        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,
        raw_payload: vec![4, item_id as u8],
        item_stack: Some(item_stack(item_id, 1)),
        tag: None,
    }
}

fn recipe_book_crafting_entry(
    id: i32,
    crafting: CraftingRecipeDisplaySummary,
) -> ProtocolRecipeBookAddEntry {
    let display_type = match crafting {
        CraftingRecipeDisplaySummary::Shapeless { .. } => RecipeDisplayType::CraftingShapeless,
        CraftingRecipeDisplaySummary::Shaped { .. } => RecipeDisplayType::CraftingShaped,
    };
    ProtocolRecipeBookAddEntry {
        contents: ProtocolRecipeDisplayEntry {
            id: RecipeDisplayId { index: id },
            display: RecipeDisplaySummary {
                display_type,
                raw_body: Vec::new(),
                crafting: Some(Box::new(crafting)),
                furnace: None,
                stonecutter: None,
                smithing: None,
            },
            group: None,
            category_id: VANILLA_RECIPE_BOOK_CATEGORY_STONECUTTER_ID,
            crafting_requirements: None,
        },
        flags: 0,
        notification: false,
        highlight: false,
    }
}

fn map_id_item_stack(item_id: i32, count: i32, map_id: i32) -> ProtocolItemStackSummary {
    let mut item = item_stack(item_id, count);
    item.component_patch.added = 1;
    item.component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID);
    item.component_patch.map_id = Some(map_id);
    item
}

fn item_stack_with_int_component_summary(
    item_id: i32,
    count: i32,
    component_type_id: i32,
    value: i32,
) -> ProtocolItemStackSummary {
    let mut item = item_stack(item_id, count);
    item.component_patch.added = 1;
    item.component_patch.added_type_ids = vec![component_type_id];
    match component_type_id {
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID => {
            item.component_patch.max_stack_size = Some(value)
        }
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID => item.component_patch.max_damage = Some(value),
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID => item.component_patch.damage = Some(value),
        VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID => item.component_patch.map_id = Some(value),
        other => panic!("unsupported int component test id {other}"),
    }
    item
}

fn item_stack_with_removed_component_summary(
    item_id: i32,
    count: i32,
    component_type_id: i32,
) -> ProtocolItemStackSummary {
    let mut stack = item_stack(item_id, count);
    stack.component_patch.removed_type_ids = vec![component_type_id];
    stack
}

fn item_attack_range(min_reach: f32, max_reach: f32) -> ItemAttackRange {
    ItemAttackRange {
        min_reach,
        max_reach,
        min_creative_reach: min_reach,
        max_creative_reach: max_reach,
        hitbox_margin: 0.125,
        mob_factor: 1.0,
    }
}

fn hashed_item_stack(item_id: i32, count: i32) -> ProtocolHashedStack {
    ProtocolHashedStack::Item(ProtocolHashedItemStack {
        item_id,
        count,
        components: ProtocolHashedComponentPatch::default(),
    })
}

fn hashed_removed_component_item_stack(
    item_id: i32,
    count: i32,
    component_type_id: i32,
) -> ProtocolHashedStack {
    ProtocolHashedStack::Item(ProtocolHashedItemStack {
        item_id,
        count,
        components: ProtocolHashedComponentPatch {
            added_components: BTreeMap::new(),
            removed_components: BTreeSet::from([component_type_id]),
        },
    })
}

fn hashed_int_component_item_stack(
    item_id: i32,
    count: i32,
    component_type_id: i32,
    value: i32,
) -> ProtocolHashedStack {
    ProtocolHashedStack::Item(ProtocolHashedItemStack {
        item_id,
        count,
        components: ProtocolHashedComponentPatch {
            added_components: BTreeMap::from([(component_type_id, hash_ops_crc32c_int(value))]),
            removed_components: BTreeSet::new(),
        },
    })
}

fn hashed_map_id_item_stack(item_id: i32, count: i32, map_id: i32) -> ProtocolHashedStack {
    ProtocolHashedStack::Item(ProtocolHashedItemStack {
        item_id,
        count,
        components: ProtocolHashedComponentPatch {
            added_components: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID,
                hash_ops_crc32c_int(map_id),
            )]),
            removed_components: BTreeSet::new(),
        },
    })
}

fn hashed_map_id_removed_component_item_stack(
    item_id: i32,
    count: i32,
    map_id: i32,
    component_type_id: i32,
) -> ProtocolHashedStack {
    ProtocolHashedStack::Item(ProtocolHashedItemStack {
        item_id,
        count,
        components: ProtocolHashedComponentPatch {
            added_components: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID,
                hash_ops_crc32c_int(map_id),
            )]),
            removed_components: BTreeSet::from([component_type_id]),
        },
    })
}

#[test]
fn hash_ops_crc32c_int_matches_guava_for_map_id() {
    assert_eq!(crc32c(b"123456789"), 0xe306_9283);
    assert_eq!(hash_ops_crc32c_int(7), -1_726_626_450);
}

fn apply_player_instabuild(store: &mut WorldStore, instabuild: bool) {
    store.apply_player_abilities(ProtocolPlayerAbilities {
        invulnerable: false,
        flying: false,
        can_fly: instabuild,
        instabuild,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
}

fn quickcraft_mask(header: i8, quickcraft_type: i8) -> i8 {
    (header & 3) | ((quickcraft_type & 3) << 2)
}

fn quick_craft_request(
    slot_num: i16,
    header: i8,
    quickcraft_type: i8,
) -> ContainerClickSlotRequest {
    ContainerClickSlotRequest {
        slot_num,
        button_num: quickcraft_mask(header, quickcraft_type),
        input: ProtocolContainerInput::QuickCraft,
    }
}

fn update_recipes(property_sets: Vec<(&str, Vec<i32>)>) -> ProtocolUpdateRecipes {
    ProtocolUpdateRecipes {
        property_sets: property_sets
            .into_iter()
            .map(|(key, item_ids)| RecipePropertySetSummary {
                key: key.to_string(),
                item_ids,
            })
            .collect(),
        stonecutter_recipes: Vec::new(),
    }
}

fn apply_item_tags(store: &mut WorldStore, tags: Vec<(&str, Vec<i32>)>) {
    store.apply_update_tags(ProtocolUpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: tags
                .into_iter()
                .map(|(tag, entries)| TagNetworkPayload {
                    tag: tag.to_string(),
                    entries,
                })
                .collect(),
        }],
    });
}

fn update_stonecutter_recipes(
    stonecutter_recipes: Vec<StonecutterSelectableRecipeSummary>,
) -> ProtocolUpdateRecipes {
    ProtocolUpdateRecipes {
        property_sets: Vec::new(),
        stonecutter_recipes,
    }
}

fn stonecutter_recipe(item_ids: Vec<i32>) -> StonecutterSelectableRecipeSummary {
    StonecutterSelectableRecipeSummary {
        input: IngredientSummary {
            tag: None,
            item_ids,
        },
        option_display: slot_display_empty(),
    }
}

fn item_stack_with_component_summary(
    item_id: i32,
    count: i32,
    component_type_id: i32,
) -> ProtocolItemStackSummary {
    let mut stack = item_stack(item_id, count);
    stack.component_patch.added = 1;
    stack.component_patch.added_type_ids = vec![component_type_id];
    stack
}

fn profiled_item_stack(item_id: i32, count: i32) -> ProtocolItemStackSummary {
    let mut stack =
        item_stack_with_component_summary(item_id, count, VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID);
    stack.component_patch.profile = Some(ResolvableProfileSummary {
        kind: ResolvableProfileKindSummary::Partial,
        uuid: Some(Uuid::from_u128(0x12345678_1234_5678_90ab_cdef12345678)),
        name: Some("Steve".to_string()),
        properties: Vec::new(),
        profile_textures: None,
        skin_patch: Default::default(),
    });
    stack
}

fn bundle_item_stack(
    item_id: i32,
    count: i32,
    bundle_contents_item_count: usize,
) -> ProtocolItemStackSummary {
    let mut stack = item_stack(item_id, count);
    stack.component_patch.bundle_contents_item_count = Some(bundle_contents_item_count);
    stack
}

fn player_slot_selection(store: &WorldStore, slot: i32) -> i32 {
    store
        .inventory()
        .player_slots
        .iter()
        .find(|state| state.slot == slot)
        .map(|state| state.local_selected_bundle_item_index)
        .unwrap()
}

fn player_slot_item(store: &WorldStore, slot: i32) -> ProtocolItemStackSummary {
    store
        .inventory()
        .player_slots
        .iter()
        .find(|state| state.slot == slot)
        .map(|state| state.item.clone())
        .unwrap_or_else(ProtocolItemStackSummary::empty)
}

fn inventory_menu_slot_item(store: &WorldStore, slot: i16) -> ProtocolItemStackSummary {
    store
        .inventory()
        .inventory_menu
        .slots
        .iter()
        .find(|state| state.slot == slot)
        .map(|state| state.item.clone())
        .unwrap_or_else(ProtocolItemStackSummary::empty)
}

fn open_container_slot_item(store: &WorldStore, slot: i16) -> ProtocolItemStackSummary {
    store
        .inventory()
        .open_container
        .as_ref()
        .unwrap()
        .slots
        .iter()
        .find(|state| state.slot == slot)
        .map(|state| state.item.clone())
        .unwrap_or_else(ProtocolItemStackSummary::empty)
}

fn container_slot_selection(store: &WorldStore, slot: i16) -> i32 {
    store
        .inventory()
        .open_container
        .as_ref()
        .unwrap()
        .slots
        .iter()
        .find(|state| state.slot == slot)
        .map(|state| state.local_selected_bundle_item_index)
        .unwrap()
}

fn merchant_offers(container_id: i32, offer_count: usize) -> ProtocolMerchantOffers {
    ProtocolMerchantOffers {
        container_id,
        offers: (0..offer_count)
            .map(|index| ProtocolMerchantOffer {
                buy_a: item_cost(42 + index as i32, 3),
                sell: item_stack(99 + index as i32, 1),
                buy_b: None,
                is_out_of_stock: false,
                uses: 1,
                max_uses: 12,
                xp: 8,
                special_price_diff: -2,
                price_multiplier: 0.05,
                demand: 6,
            })
            .collect(),
        villager_level: 3,
        villager_xp: 120,
        show_progress: true,
        can_restock: false,
    }
}

fn item_cost(item_id: i32, count: i32) -> ProtocolItemCostSummary {
    ProtocolItemCostSummary {
        item_id,
        count,
        component_predicate: Default::default(),
    }
}

#[test]
fn inventory_state_serde_shape_survives_world_store_json_round_trip() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 36,
        item: item_stack(42, 3),
    });
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(7, 1),
    });
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 5,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X1_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });

    let value = serde_json::to_value(&store).unwrap();
    let inventory = value
        .get("inventory")
        .and_then(|value| value.as_object())
        .expect("world store serializes the sub-store under the `inventory` key");
    let mut keys: Vec<&str> = inventory.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "cursor_item",
            "inventory_menu",
            "local_inventory_open",
            "open_container",
            "player_slots",
        ]
    );
    let open_container = inventory
        .get("open_container")
        .and_then(|value| value.as_object())
        .expect("open container serializes as an object");
    let mut container_keys: Vec<&str> = open_container.keys().map(String::as_str).collect();
    container_keys.sort_unstable();
    assert_eq!(
        container_keys,
        [
            "container_id",
            "data_values",
            "menu_type_id",
            "merchant_offers",
            "mount",
            "slots",
            "state_id",
            "title",
        ]
    );

    let restored: WorldStore = serde_json::from_value(value).unwrap();
    assert_eq!(restored.inventory(), store.inventory());
}

#[test]
fn open_screen_exact_title_is_stored_and_survives_set_content() {
    let title = bbb_protocol::RichTextComponent::try_from_component_nbt(
        bbb_protocol::NbtSummaryValue::Compound(vec![
            bbb_protocol::NbtSummaryEntry {
                name: "text".to_string(),
                value: bbb_protocol::NbtSummaryValue::String("Chest".to_string()),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "bold".to_string(),
                value: bbb_protocol::NbtSummaryValue::Byte(1),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "color".to_string(),
                value: bbb_protocol::NbtSummaryValue::String("#ff5555".to_string()),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "click_event".to_string(),
                value: bbb_protocol::NbtSummaryValue::Compound(vec![
                    bbb_protocol::NbtSummaryEntry {
                        name: "action".to_string(),
                        value: bbb_protocol::NbtSummaryValue::String("run_command".to_string()),
                    },
                    bbb_protocol::NbtSummaryEntry {
                        name: "command".to_string(),
                        value: bbb_protocol::NbtSummaryValue::String("/menu".to_string()),
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
            command: "/menu".to_string(),
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
