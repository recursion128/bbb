use super::*;
use crate::entities::{
    VANILLA_ENTITY_TYPE_DONKEY_ID, VANILLA_ENTITY_TYPE_HORSE_ID, VANILLA_ENTITY_TYPE_LLAMA_ID,
    VANILLA_ENTITY_TYPE_NAUTILUS_ID,
};
use bbb_protocol::packets::{
    AddEntity as ProtocolAddEntity, CraftingRecipeDisplaySummary,
    EntityDataValue as ProtocolEntityDataValue, EntityDataValueKind, IngredientSummary,
    MountScreenOpen as ProtocolMountScreenOpen, PlayerAbilities as ProtocolPlayerAbilities,
    PlayerExperience as ProtocolPlayerExperience, RecipeBookAdd as ProtocolRecipeBookAdd,
    RecipeBookAddEntry as ProtocolRecipeBookAddEntry,
    RecipeDisplayEntry as ProtocolRecipeDisplayEntry, RecipeDisplayId, RecipeDisplaySummary,
    RecipeDisplayType, RecipePropertySetSummary, RegistryTags,
    SetEntityData as ProtocolSetEntityData, SlotDisplaySummary, StonecutterSelectableRecipeSummary,
    TagNetworkPayload, UpdateRecipes as ProtocolUpdateRecipes, UpdateTags as ProtocolUpdateTags,
    UseEffectsSummary as ProtocolUseEffectsSummary, Vec3d as ProtocolVec3d,
};
use uuid::Uuid;

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
        menu_type_id: 2,
        title: "Chest".to_string(),
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
    assert_eq!(container.menu_type_id, Some(2));
    assert_eq!(container.title.as_deref(), Some("Chest"));
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

    assert!(store.apply_container_close(ProtocolContainerClose { container_id: 7 }));
    assert!(store.inventory().open_container.is_none());
    assert_eq!(store.open_container_data_value(2), None);
    assert!(!store.apply_container_close(ProtocolContainerClose { container_id: 99 }));

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
    assert_eq!(
        mount_visibility_for_entity(
            VANILLA_ENTITY_TYPE_DONKEY_ID,
            vec![protocol_byte_data(
                VANILLA_MOUNT_TAME_FLAGS_DATA_ID,
                VANILLA_ABSTRACT_HORSE_TAME_FLAG,
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
                protocol_bool_data(VANILLA_AGEABLE_MOB_BABY_DATA_ID, true),
                protocol_byte_data(
                    VANILLA_MOUNT_TAME_FLAGS_DATA_ID,
                    VANILLA_ABSTRACT_HORSE_TAME_FLAG,
                ),
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
            vec![protocol_byte_data(
                VANILLA_MOUNT_TAME_FLAGS_DATA_ID,
                VANILLA_TAMABLE_ANIMAL_TAME_FLAG,
            )],
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
        slot: PLAYER_CHEST_EQUIPMENT_SLOT,
        item: item_stack(VANILLA_ELYTRA_ITEM_ID, 1),
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
        item_stack(VANILLA_ELYTRA_ITEM_ID, 1)
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
        slot: PLAYER_OFFHAND_SLOT,
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
        slot: PLAYER_OFFHAND_SLOT,
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
        item: item_stack_with_component_summary(99, 1, VANILLA_PIERCING_WEAPON_COMPONENT_ID),
    });

    assert!(store.local_selected_main_hand_has_piercing_weapon());
}

#[test]
fn local_selected_main_hand_has_piercing_weapon_removed_component_overrides_default_and_added() {
    let mut store = WorldStore::new();
    store.set_default_piercing_weapon_item_ids(BTreeSet::from([42]));
    let mut item = item_stack_with_component_summary(42, 1, VANILLA_PIERCING_WEAPON_COMPONENT_ID);
    item.component_patch.removed_type_ids = vec![VANILLA_PIERCING_WEAPON_COMPONENT_ID];
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
    patched.component_patch.added_type_ids = vec![VANILLA_ATTACK_RANGE_COMPONENT_ID];
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
fn local_selected_main_hand_attack_range_removed_component_overrides_default_and_added() {
    let mut store = WorldStore::new();
    store.set_default_item_attack_ranges(BTreeMap::from([(42, item_attack_range(2.0, 4.5))]));
    let mut item = item_stack(42, 1);
    item.component_patch.added = 1;
    item.component_patch.added_type_ids = vec![VANILLA_ATTACK_RANGE_COMPONENT_ID];
    item.component_patch.attack_range = Some(ProtocolAttackRangeSummary {
        min_reach: 1.0,
        max_reach: 2.25,
        min_creative_reach: 1.0,
        max_creative_reach: 3.0,
        hitbox_margin: 0.25,
        mob_factor: 0.75,
    });
    item.component_patch.removed_type_ids = vec![VANILLA_ATTACK_RANGE_COMPONENT_ID];
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
    patched.component_patch.added_type_ids = vec![VANILLA_USE_EFFECTS_COMPONENT_ID];
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
    item.component_patch.removed_type_ids = vec![VANILLA_USE_EFFECTS_COMPONENT_ID];
    store.apply_set_player_inventory(ProtocolSetPlayerInventory { slot: 0, item });
    store.set_local_using_item(true);

    assert_eq!(
        store.local_using_item_use_effects(),
        Some(ItemUseEffects::default())
    );
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
fn local_player_has_equipped_elytra_true_for_non_empty_elytra_in_chest_slot() {
    let mut store = WorldStore::new();

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: PLAYER_CHEST_EQUIPMENT_SLOT,
        item: item_stack(VANILLA_ELYTRA_ITEM_ID, 1),
    });

    assert!(store.local_player_has_equipped_elytra());
}

#[test]
fn local_player_has_equipped_elytra_false_when_chest_slot_is_missing() {
    let store = WorldStore::new();

    assert!(!store.local_player_has_equipped_elytra());
}

#[test]
fn local_player_has_equipped_elytra_false_when_elytra_is_in_wrong_slot() {
    let mut store = WorldStore::new();

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: PLAYER_CHEST_EQUIPMENT_SLOT - 1,
        item: item_stack(VANILLA_ELYTRA_ITEM_ID, 1),
    });

    assert!(!store.local_player_has_equipped_elytra());
}

#[test]
fn local_player_has_equipped_elytra_false_for_empty_or_count_zero_stack() {
    let mut store = WorldStore::new();

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: PLAYER_CHEST_EQUIPMENT_SLOT,
        item: ProtocolItemStackSummary::empty(),
    });
    assert!(!store.local_player_has_equipped_elytra());

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: PLAYER_CHEST_EQUIPMENT_SLOT,
        item: item_stack(VANILLA_ELYTRA_ITEM_ID, 0),
    });
    assert!(!store.local_player_has_equipped_elytra());
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
        menu_type_id: 2,
        title: "Chest".to_string(),
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
        menu_type_id: 18,
        title: "Merchant".to_string(),
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
        title: "Merchant".to_string(),
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
        title: "Merchant".to_string(),
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
fn build_container_click_slot_uses_open_container_state_and_cursor_item() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: 2,
        title: "Chest".to_string(),
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
fn apply_local_container_pickup_all_rejects_non_inventory_menu() {
    let mut store = WorldStore::new();
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(42, 5),
    });
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: 1,
        title: "Chest".to_string(),
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items: vec![ProtocolItemStackSummary::empty(), item_stack(42, 3)],
        carried_item: item_stack(42, 5),
    });

    assert_eq!(
        store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 0,
                button_num: 0,
                input: ProtocolContainerInput::PickupAll,
            })
            .unwrap_err(),
        ContainerClickBuildError::UnsupportedLocalClickInput(ProtocolContainerInput::PickupAll)
    );
    assert_eq!(
        store.inventory().open_container.as_ref().unwrap().slots[1].item,
        item_stack(42, 3)
    );
    assert_eq!(store.inventory().cursor_item, item_stack(42, 5));
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
fn apply_local_container_quick_craft_rejects_non_inventory_menu() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: 1,
        title: "Chest".to_string(),
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items: vec![ProtocolItemStackSummary::empty()],
        carried_item: item_stack(42, 8),
    });

    assert_eq!(
        store
            .apply_local_container_click_slot(quick_craft_request(
                0,
                QUICKCRAFT_HEADER_START,
                QUICKCRAFT_TYPE_CHARITABLE,
            ))
            .unwrap_err(),
        ContainerClickBuildError::UnsupportedLocalClickInput(ProtocolContainerInput::QuickCraft)
    );
    assert_eq!(
        store.inventory().open_container.as_ref().unwrap().slots[0].item,
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(store.inventory().cursor_item, item_stack(42, 8));
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
        player_slot_item(&store, PLAYER_CHEST_EQUIPMENT_SLOT),
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
        player_slot_item(&store, PLAYER_OFFHAND_SLOT),
        item_stack(43, 1)
    );
}

#[test]
fn apply_local_container_quick_move_uses_inventory_fallback_when_equipment_slot_is_occupied() {
    let mut store = WorldStore::new();
    store.set_default_item_equipment_slots(BTreeMap::from([(42, ItemEquipmentSlot::Chest)]));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: PLAYER_CHEST_EQUIPMENT_SLOT,
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
        player_slot_item(&store, PLAYER_CHEST_EQUIPMENT_SLOT),
        item_stack(99, 1)
    );
    assert_eq!(
        player_slot_item(&store, 9),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(player_slot_item(&store, 0), item_stack(42, 1));
}

#[test]
fn apply_local_container_pickup_result_consumes_crafting_inputs_once() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
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
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 1));
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
    assert_eq!(store.inventory().cursor_item, item_stack(90, 1));
}

#[test]
fn apply_local_container_quick_move_result_repeats_while_inputs_remain() {
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
            (1, ProtocolHashedStack::Empty),
            (2, ProtocolHashedStack::Empty),
            (4, ProtocolHashedStack::Empty),
            (44, hashed_item_stack(90, 6)),
        ])
    );
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        inventory_menu_slot_item(&store, 0),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        inventory_menu_slot_item(&store, 1),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        inventory_menu_slot_item(&store, 2),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        inventory_menu_slot_item(&store, 4),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(inventory_menu_slot_item(&store, 44), item_stack(90, 6));
    assert_eq!(player_slot_item(&store, 8), item_stack(90, 6));
}

#[test]
fn apply_local_container_quick_move_result_consumes_after_partial_transfer() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    store.set_default_item_max_stack_sizes(BTreeMap::from([(90, 1)]));
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 2);
    items[1] = item_stack(42, 2);
    items[2] = item_stack(43, 2);
    items[4] = item_stack(44, 2);
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
            (1, ProtocolHashedStack::Empty),
            (2, ProtocolHashedStack::Empty),
            (4, ProtocolHashedStack::Empty),
            (43, hashed_item_stack(90, 1)),
            (44, hashed_item_stack(90, 1)),
        ])
    );
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        inventory_menu_slot_item(&store, 0),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        inventory_menu_slot_item(&store, 1),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        inventory_menu_slot_item(&store, 2),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        inventory_menu_slot_item(&store, 4),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(inventory_menu_slot_item(&store, 43), item_stack(90, 1));
    assert_eq!(inventory_menu_slot_item(&store, 44), item_stack(90, 1));
    assert_eq!(player_slot_item(&store, 7), item_stack(90, 1));
    assert_eq!(player_slot_item(&store, 8), item_stack(90, 1));
}

#[test]
fn apply_local_container_click_recomputes_shaped_recipe_result_from_recipe_book() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    store.apply_recipe_book_add(ProtocolRecipeBookAdd {
        entries: vec![recipe_book_crafting_entry(
            7,
            CraftingRecipeDisplaySummary::Shaped {
                width: 2,
                height: 1,
                ingredients: vec![slot_display_item(42), slot_display_item(43)],
                result: slot_display_item_stack(90, 2),
                crafting_station: slot_display_empty(),
            },
        )],
        replace: true,
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
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
        BTreeMap::from([(0, hashed_item_stack(90, 2)), (2, hashed_item_stack(43, 1)),])
    );
    assert_eq!(pickup.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(inventory_menu_slot_item(&store, 0), item_stack(90, 2));
    assert_eq!(inventory_menu_slot_item(&store, 1), item_stack(42, 1));
    assert_eq!(inventory_menu_slot_item(&store, 2), item_stack(43, 1));
    assert_eq!(
        store.inventory().cursor_item,
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_container_click_recomputes_shapeless_recipe_result_from_recipe_book() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    store.apply_recipe_book_add(ProtocolRecipeBookAdd {
        entries: vec![recipe_book_crafting_entry(
            8,
            CraftingRecipeDisplaySummary::Shapeless {
                ingredients: vec![slot_display_item(42), slot_display_item(43)],
                result: slot_display_item(91),
                crafting_station: slot_display_empty(),
            },
        )],
        replace: true,
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[4] = item_stack(43, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: INVENTORY_MENU_CONTAINER_ID,
        state_id: 12,
        items,
        carried_item: item_stack(42, 1),
    });
    assert!(store.open_local_inventory());

    let pickup = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 1,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(
        pickup.changed_slots,
        BTreeMap::from([(0, hashed_item_stack(91, 1)), (1, hashed_item_stack(42, 1)),])
    );
    assert_eq!(pickup.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(inventory_menu_slot_item(&store, 0), item_stack(91, 1));
    assert_eq!(inventory_menu_slot_item(&store, 1), item_stack(42, 1));
    assert_eq!(inventory_menu_slot_item(&store, 4), item_stack(43, 1));
}

#[test]
fn apply_local_container_click_recomputes_shaped_recipe_result_from_requirements() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    apply_item_tags(&mut store, vec![("minecraft:planks", vec![42, 43])]);
    store.apply_recipe_book_add(ProtocolRecipeBookAdd {
        entries: vec![recipe_book_crafting_entry_with_requirements(
            9,
            CraftingRecipeDisplaySummary::Shaped {
                width: 2,
                height: 1,
                ingredients: vec![slot_display_tag("minecraft:planks"), slot_display_item(44)],
                result: slot_display_item_stack(90, 4),
                crafting_station: slot_display_empty(),
            },
            vec![
                ingredient_tag("minecraft:planks"),
                ingredient_items(vec![44]),
            ],
        )],
        replace: true,
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[1] = item_stack(44, 1);
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
        BTreeMap::from([(0, hashed_item_stack(90, 4)), (2, hashed_item_stack(43, 1)),])
    );
    assert_eq!(pickup.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(inventory_menu_slot_item(&store, 0), item_stack(90, 4));
    assert_eq!(inventory_menu_slot_item(&store, 1), item_stack(44, 1));
    assert_eq!(inventory_menu_slot_item(&store, 2), item_stack(43, 1));
}

#[test]
fn apply_local_container_click_recomputes_shapeless_recipe_result_from_requirements() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    apply_item_tags(&mut store, vec![("minecraft:dyes", vec![55])]);
    store.apply_recipe_book_add(ProtocolRecipeBookAdd {
        entries: vec![recipe_book_crafting_entry_with_requirements(
            10,
            CraftingRecipeDisplaySummary::Shapeless {
                ingredients: vec![slot_display_item(999), slot_display_tag("minecraft:dyes")],
                result: slot_display_item(91),
                crafting_station: slot_display_empty(),
            },
            vec![
                ingredient_items(vec![42, 55]),
                ingredient_tag("minecraft:dyes"),
            ],
        )],
        replace: true,
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[4] = item_stack(55, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: INVENTORY_MENU_CONTAINER_ID,
        state_id: 12,
        items,
        carried_item: item_stack(42, 1),
    });
    assert!(store.open_local_inventory());

    let pickup = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 1,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(
        pickup.changed_slots,
        BTreeMap::from([(0, hashed_item_stack(91, 1)), (1, hashed_item_stack(42, 1)),])
    );
    assert_eq!(pickup.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(inventory_menu_slot_item(&store, 0), item_stack(91, 1));
    assert_eq!(inventory_menu_slot_item(&store, 1), item_stack(42, 1));
    assert_eq!(inventory_menu_slot_item(&store, 4), item_stack(55, 1));
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
fn apply_local_container_result_with_default_remainder_requires_server_authority() {
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
fn apply_local_container_result_with_recipe_specific_remainder_requires_server_authority() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    store.set_recipe_specific_crafting_remainder_item_ids(BTreeSet::from([42]));
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
    assert_eq!(inventory_menu_slot_item(&store, 0), item_stack(90, 1));
    assert_eq!(inventory_menu_slot_item(&store, 1), item_stack(42, 2));
    assert_eq!(inventory_menu_slot_item(&store, 5), item_stack(43, 2));
    assert_eq!(
        inventory_menu_slot_item(&store, 44),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_generic_container_quick_move_moves_chest_to_player_reverse() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: 5,
        title: "Large Chest".to_string(),
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
        menu_type_id: 1,
        title: "Chest".to_string(),
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
        title: "Dispenser".to_string(),
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
        title: "Dispenser".to_string(),
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
        title: "Crafting".to_string(),
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
        title: "Crafting".to_string(),
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
fn apply_local_crafting_menu_result_quick_move_consumes_single_inputs() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: "Crafting".to_string(),
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
fn apply_local_crafting_menu_result_quick_move_repeats_until_inputs_empty() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: "Crafting".to_string(),
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
            (1, ProtocolHashedStack::Empty),
            (5, ProtocolHashedStack::Empty),
            (45, hashed_item_stack(90, 2)),
        ])
    );
    let slots = &store.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[1].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[5].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[45].item, item_stack(90, 2));
}

#[test]
fn apply_local_crafting_menu_result_pickup_leaves_result_when_inputs_remain() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: "Crafting".to_string(),
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
        BTreeMap::from([(1, hashed_item_stack(42, 1)), (5, hashed_item_stack(43, 1))])
    );
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 1));
    let slots = &store.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, item_stack(90, 1));
    assert_eq!(slots[1].item, item_stack(42, 1));
    assert_eq!(slots[5].item, item_stack(43, 1));
    assert_eq!(store.inventory().cursor_item, item_stack(90, 1));
}

#[test]
fn apply_local_crafting_menu_result_pickup_consumes_single_inputs_to_cursor() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: "Crafting".to_string(),
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
fn apply_local_crafting_menu_result_keeps_remainders_server_authoritative() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::from([(42, 43)]));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: "Crafting".to_string(),
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
    let slots = &store.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, item_stack(90, 1));
    assert_eq!(slots[1].item, item_stack(42, 1));
}

#[test]
fn apply_local_crafting_menu_result_keeps_recipe_specific_remainders_server_authoritative() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    store.set_recipe_specific_crafting_remainder_item_ids(BTreeSet::from([42]));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: "Crafting".to_string(),
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
    let slots = &store.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, item_stack(90, 1));
    assert_eq!(slots[1].item, item_stack(42, 2));
    assert_eq!(slots[5].item, item_stack(43, 2));
}

#[test]
fn apply_local_crafter_quick_move_moves_grid_to_player_backwards() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTER_ID,
        title: "Crafter".to_string(),
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
        menu_type_id: VANILLA_MENU_TYPE_CRAFTER_ID,
        title: "Crafter".to_string(),
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
        menu_type_id: VANILLA_MENU_TYPE_CRAFTER_ID,
        title: "Crafter".to_string(),
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
        title: "Anvil".to_string(),
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
fn apply_local_anvil_result_quick_move_consumes_single_input_when_cost_allows() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: "Anvil".to_string(),
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
        title: "Anvil".to_string(),
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
        title: "Anvil".to_string(),
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
        title: "Anvil".to_string(),
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
        title: "Anvil".to_string(),
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
        title: "Beacon".to_string(),
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
        title: "Beacon".to_string(),
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
        title: "Beacon".to_string(),
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
        title: "Beacon".to_string(),
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
        title: "Beacon".to_string(),
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
        title: "Anvil".to_string(),
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
fn apply_local_loom_result_quick_move_consumes_single_banner_and_dye() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
        title: "Loom".to_string(),
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
        title: "Loom".to_string(),
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
        title: "Loom".to_string(),
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
        title: "Loom".to_string(),
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
        title: "Loom".to_string(),
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
        title: "Loom".to_string(),
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
        title: "Merchant".to_string(),
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
        title: "Merchant".to_string(),
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
        title: "Merchant".to_string(),
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
        title: "Merchant".to_string(),
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
        title: "Merchant".to_string(),
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
        title: "Merchant".to_string(),
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
        title: "Merchant".to_string(),
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
        title: "Merchant".to_string(),
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
        title: "Merchant".to_string(),
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
    cost.component_predicate.component_count = 1;
    cost.component_predicate.component_type_ids = vec![10];
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
        title: "Merchant".to_string(),
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
        title: "Merchant".to_string(),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); MERCHANT_TOTAL_SLOT_COUNT as usize];
    items[MERCHANT_PLAYER_MAIN_START as usize] = item_stack(42, 5);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 15,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    let mut cost = item_cost(42, 1);
    cost.component_predicate.component_count = 1;
    cost.component_predicate.component_type_ids = vec![10];
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
        item_stack(42, 5)
    );
}

#[test]
fn apply_local_enchantment_quick_move_moves_menu_slots_to_player_reverse() {
    const ENCHANTMENT_TOTAL_SLOT_COUNT: usize = 38;

    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ENCHANTMENT_ID,
        title: "Enchanting Table".to_string(),
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
        title: "Enchanting Table".to_string(),
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
        title: "Smithing".to_string(),
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
        title: "Smithing".to_string(),
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
        title: "Smithing".to_string(),
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
        title: "Smithing".to_string(),
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
        title: "Smithing".to_string(),
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
        title: "Smithing".to_string(),
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
        title: "Cartography Table".to_string(),
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
        title: "Cartography Table".to_string(),
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
        title: "Cartography Table".to_string(),
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
        title: "Cartography Table".to_string(),
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
    store.set_cartography_additional_item_ids(BTreeSet::from([43, 44, 45]));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
        title: "Cartography Table".to_string(),
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
        title: "Cartography Table".to_string(),
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
fn cartography_table_map_id_unknown_component_requires_server_authority() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
        title: "Cartography Table".to_string(),
    });
    let mut items =
        vec![ProtocolItemStackSummary::empty(); CARTOGRAPHY_TABLE_TOTAL_SLOT_COUNT as usize];
    let mut map_stack = map_id_item_stack(42, 1, 7);
    map_stack.component_patch.added = 2;
    map_stack
        .component_patch
        .added_type_ids
        .push(VANILLA_MAX_DAMAGE_COMPONENT_ID);
    map_stack.component_patch.max_damage = Some(100);
    items[CARTOGRAPHY_TABLE_PLAYER_MAIN_START as usize] = map_stack.clone();
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: CARTOGRAPHY_TABLE_PLAYER_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::QuickMove
        ))
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_PLAYER_MAIN_START),
        map_stack
    );
}

#[test]
fn apply_local_stonecutter_quick_move_moves_input_slot_to_player_forward() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: "Stonecutter".to_string(),
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
        title: "Stonecutter".to_string(),
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
        title: "Stonecutter".to_string(),
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
        title: "Stonecutter".to_string(),
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
        title: "Stonecutter".to_string(),
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
        title: "Stonecutter".to_string(),
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
        title: "Stonecutter".to_string(),
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
        title: "Brewing Stand".to_string(),
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
        title: "Brewing Stand".to_string(),
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
        title: "Grindstone".to_string(),
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
        title: "Grindstone".to_string(),
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
        title: "Grindstone".to_string(),
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
        title: "Grindstone".to_string(),
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
        title: "Grindstone".to_string(),
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
        title: "Grindstone".to_string(),
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
        title: "Grindstone".to_string(),
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
        title: "Grindstone".to_string(),
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
        title: "Hopper".to_string(),
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
        title: "Hopper".to_string(),
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
        title: "Shulker Box".to_string(),
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
        title: "Shulker Box".to_string(),
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
        title: "Furnace".to_string(),
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
        title: "Furnace".to_string(),
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
        title: "Blast Furnace".to_string(),
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
        title: "Smoker".to_string(),
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
        title: "Furnace".to_string(),
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
        menu_type_id: 99,
        title: "Unsupported".to_string(),
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
fn apply_local_container_throw_rejects_non_inventory_menu() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: 1,
        title: "Chest".to_string(),
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items: vec![item_stack(42, 3)],
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert_eq!(
        store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 0,
                button_num: 0,
                input: ProtocolContainerInput::Throw,
            })
            .unwrap_err(),
        ContainerClickBuildError::UnsupportedLocalClickInput(ProtocolContainerInput::Throw)
    );
    assert_eq!(
        store.inventory().open_container.as_ref().unwrap().slots[0].item,
        item_stack(42, 3)
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
        player_slot_item(&store, PLAYER_OFFHAND_SLOT),
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
fn apply_local_container_swap_requires_empty_cursor_and_inventory_menu() {
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
        menu_type_id: 1,
        title: "Chest".to_string(),
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items: vec![item_stack(42, 3)],
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert_eq!(
        store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 0,
                button_num: 0,
                input: ProtocolContainerInput::Swap,
            })
            .unwrap_err(),
        ContainerClickBuildError::UnsupportedLocalClickInput(ProtocolContainerInput::Swap)
    );
}

#[test]
fn apply_local_container_clone_copies_slot_stack_to_cursor_at_max_count() {
    let mut store = WorldStore::new();
    store.set_default_item_max_stack_sizes(BTreeMap::from([(42, 16)]));
    apply_player_instabuild(&mut store, true);
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: 1,
        title: "Chest".to_string(),
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
        menu_type_id: 1,
        title: "Chest".to_string(),
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
        menu_type_id: 1,
        title: "Chest".to_string(),
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
        serializer_id: 0,
        value: EntityDataValueKind::Byte(value),
    }
}

fn protocol_bool_data(data_id: u8, value: bool) -> ProtocolEntityDataValue {
    ProtocolEntityDataValue {
        data_id,
        serializer_id: 8,
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

fn slot_display_empty() -> SlotDisplaySummary {
    SlotDisplaySummary {
        display_type_id: 0,
        raw_payload: vec![0],
        item_stack: None,
    }
}

fn slot_display_item(item_id: i32) -> SlotDisplaySummary {
    SlotDisplaySummary {
        display_type_id: 4,
        raw_payload: vec![4, item_id as u8],
        item_stack: Some(item_stack(item_id, 1)),
    }
}

fn slot_display_item_stack(item_id: i32, count: i32) -> SlotDisplaySummary {
    SlotDisplaySummary {
        display_type_id: 5,
        raw_payload: vec![5, item_id as u8, count as u8, 0, 0],
        item_stack: Some(item_stack(item_id, count)),
    }
}

fn slot_display_tag(tag: &str) -> SlotDisplaySummary {
    SlotDisplaySummary {
        display_type_id: 6,
        raw_payload: tag.as_bytes().to_vec(),
        item_stack: None,
    }
}

fn recipe_book_crafting_entry(
    id: i32,
    crafting: CraftingRecipeDisplaySummary,
) -> ProtocolRecipeBookAddEntry {
    recipe_book_crafting_entry_with_optional_requirements(id, crafting, None)
}

fn recipe_book_crafting_entry_with_requirements(
    id: i32,
    crafting: CraftingRecipeDisplaySummary,
    crafting_requirements: Vec<IngredientSummary>,
) -> ProtocolRecipeBookAddEntry {
    recipe_book_crafting_entry_with_optional_requirements(id, crafting, Some(crafting_requirements))
}

fn recipe_book_crafting_entry_with_optional_requirements(
    id: i32,
    crafting: CraftingRecipeDisplaySummary,
    crafting_requirements: Option<Vec<IngredientSummary>>,
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
                crafting: Some(crafting),
            },
            group: None,
            category_id: 10,
            crafting_requirements,
        },
        flags: 0,
        notification: false,
        highlight: false,
    }
}

fn ingredient_items(item_ids: Vec<i32>) -> IngredientSummary {
    IngredientSummary {
        tag: None,
        item_ids,
    }
}

fn ingredient_tag(tag: &str) -> IngredientSummary {
    IngredientSummary {
        tag: Some(tag.to_string()),
        item_ids: Vec::new(),
    }
}

fn map_id_item_stack(item_id: i32, count: i32, map_id: i32) -> ProtocolItemStackSummary {
    let mut item = item_stack(item_id, count);
    item.component_patch.added = 1;
    item.component_patch
        .added_type_ids
        .push(VANILLA_MAP_ID_COMPONENT_ID);
    item.component_patch.map_id = Some(map_id);
    item
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

fn hashed_map_id_item_stack(item_id: i32, count: i32, map_id: i32) -> ProtocolHashedStack {
    ProtocolHashedStack::Item(ProtocolHashedItemStack {
        item_id,
        count,
        components: ProtocolHashedComponentPatch {
            added_components: BTreeMap::from([(
                VANILLA_MAP_ID_COMPONENT_ID,
                hash_ops_crc32c_int(map_id),
            )]),
            removed_components: BTreeSet::new(),
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
        option_display: SlotDisplaySummary {
            display_type_id: 0,
            raw_payload: Vec::new(),
            item_stack: None,
        },
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
