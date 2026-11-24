use super::*;

pub(super) fn mount_visibility_for_entity(
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

pub(super) fn protocol_add_entity_with_type(id: i32, entity_type_id: i32) -> ProtocolAddEntity {
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

pub(super) fn protocol_byte_data(data_id: u8, value: i8) -> ProtocolEntityDataValue {
    ProtocolEntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        value: EntityDataValueKind::Byte(value),
    }
}

pub(super) fn protocol_bool_data(data_id: u8, value: bool) -> ProtocolEntityDataValue {
    ProtocolEntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        value: EntityDataValueKind::Boolean(value),
    }
}

pub(super) fn equippable_with_camera_overlay(
    slot_id: i32,
    camera_overlay: Option<&str>,
) -> bbb_protocol::packets::EquippableSummary {
    bbb_protocol::packets::EquippableSummary {
        slot_id,
        equip_sound: bbb_protocol::packets::SoundEventSummary {
            registry_id: None,
            sound_id: Some("minecraft:item.armor.equip_generic".to_string()),
            fixed_range_bits: None,
        },
        asset_id: None,
        camera_overlay: camera_overlay.map(str::to_string),
        allowed_entities: None,
        dispensable: true,
        swappable: true,
        damage_on_hurt: true,
        equip_on_interact: false,
        can_be_sheared: false,
        shearing_sound: bbb_protocol::packets::SoundEventSummary {
            registry_id: None,
            sound_id: Some("minecraft:item.shears.snip".to_string()),
            fixed_range_bits: None,
        },
    }
}

pub(super) fn item_stack(item_id: i32, count: i32) -> ProtocolItemStackSummary {
    ProtocolItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    }
}

pub(super) fn install_loom_banner_pattern_registry(store: &mut WorldStore) {
    store.registries.contents.insert(
        BANNER_PATTERN_REGISTRY.to_string(),
        crate::RegistryContentState::from_entries(
            BANNER_PATTERN_REGISTRY,
            [
                "example:custom",
                "minecraft:stripe",
                "minecraft:flower",
                "minecraft:base",
            ]
            .into_iter()
            .map(crate::RegistryPacketEntry::stub)
            .collect(),
        ),
    );
    store.registries.tags.insert(
        BANNER_PATTERN_REGISTRY.to_string(),
        RegistryTagState::from_materialized(
            BANNER_PATTERN_REGISTRY.to_string(),
            BTreeMap::from([
                (LOOM_NO_ITEM_REQUIRED_PATTERN_TAG.to_string(), vec![3, 0, 1]),
                ("example:item_patterns".to_string(), vec![2, 0]),
            ]),
        ),
    );
}

pub(super) fn open_loom_with_pattern_stack(
    store: &mut WorldStore,
    pattern_stack: ProtocolItemStackSummary,
) {
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

pub(super) fn slot_display_empty() -> SlotDisplaySummary {
    SlotDisplaySummary {
        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
        raw_payload: vec![0],
        item_stack: None,
        tag: None,
    }
}

pub(super) fn slot_display_item(item_id: i32) -> SlotDisplaySummary {
    SlotDisplaySummary {
        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,
        raw_payload: vec![4, item_id as u8],
        item_stack: Some(item_stack(item_id, 1)),
        tag: None,
    }
}

pub(super) fn recipe_book_crafting_entry(
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

pub(super) fn map_id_item_stack(item_id: i32, count: i32, map_id: i32) -> ProtocolItemStackSummary {
    let mut item = item_stack(item_id, count);
    item.component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID);
    item.component_patch.map_id = Some(map_id);
    item
}

pub(super) fn item_stack_with_int_component_summary(
    item_id: i32,
    count: i32,
    component_type_id: i32,
    value: i32,
) -> ProtocolItemStackSummary {
    let mut item = item_stack(item_id, count);
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

pub(super) fn item_stack_with_removed_component_summary(
    item_id: i32,
    count: i32,
    component_type_id: i32,
) -> ProtocolItemStackSummary {
    let mut stack = item_stack(item_id, count);
    stack.component_patch.removed_type_ids = vec![component_type_id];
    stack
}

pub(super) fn item_attack_range(min_reach: f32, max_reach: f32) -> ItemAttackRange {
    ItemAttackRange {
        min_reach,
        max_reach,
        min_creative_reach: min_reach,
        max_creative_reach: max_reach,
        hitbox_margin: 0.125,
        mob_factor: 1.0,
    }
}

pub(super) fn hashed_item_stack(item_id: i32, count: i32) -> ProtocolHashedStack {
    ProtocolHashedStack::Item(ProtocolHashedItemStack {
        item_id,
        count,
        components: ProtocolHashedComponentPatch::default(),
    })
}

pub(super) fn hashed_removed_component_item_stack(
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

pub(super) fn hashed_int_component_item_stack(
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

pub(super) fn hashed_map_id_item_stack(
    item_id: i32,
    count: i32,
    map_id: i32,
) -> ProtocolHashedStack {
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

pub(super) fn hashed_map_id_removed_component_item_stack(
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

pub(super) fn apply_player_instabuild(store: &mut WorldStore, instabuild: bool) {
    store.apply_player_abilities(ProtocolPlayerAbilities {
        invulnerable: false,
        flying: false,
        can_fly: instabuild,
        instabuild,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
}

pub(super) fn quickcraft_mask(header: i8, quickcraft_type: i8) -> i8 {
    (header & 3) | ((quickcraft_type & 3) << 2)
}

pub(super) fn quick_craft_request(
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

pub(super) fn update_recipes(property_sets: Vec<(&str, Vec<i32>)>) -> ProtocolUpdateRecipes {
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

pub(super) fn apply_item_tags(store: &mut WorldStore, tags: Vec<(&str, Vec<i32>)>) {
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

pub(super) fn update_stonecutter_recipes(
    stonecutter_recipes: Vec<StonecutterSelectableRecipeSummary>,
) -> ProtocolUpdateRecipes {
    ProtocolUpdateRecipes {
        property_sets: Vec::new(),
        stonecutter_recipes,
    }
}

pub(super) fn stonecutter_recipe(item_ids: Vec<i32>) -> StonecutterSelectableRecipeSummary {
    StonecutterSelectableRecipeSummary {
        input: IngredientSummary {
            tag: None,
            item_ids,
        },
        option_display: slot_display_empty(),
    }
}

pub(super) fn item_stack_with_component_summary(
    item_id: i32,
    count: i32,
    component_type_id: i32,
) -> ProtocolItemStackSummary {
    let mut stack = item_stack(item_id, count);
    stack.component_patch.added_type_ids = vec![component_type_id];
    stack
}

pub(super) fn profiled_item_stack(item_id: i32, count: i32) -> ProtocolItemStackSummary {
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

pub(super) fn bundle_item_stack(
    item_id: i32,
    count: i32,
    contents_count: usize,
) -> ProtocolItemStackSummary {
    let mut stack = item_stack(item_id, count);
    stack
        .component_patch
        .added_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID);
    stack.component_patch.bundle_contents_items = (0..contents_count)
        .map(|_| bbb_protocol::packets::ItemStackTemplateSummary {
            item_id: bbb_protocol::item_types::VANILLA_ITEM_STONE_ID,
            count: 1,
            component_patch: Default::default(),
        })
        .collect();
    stack
}

pub(super) fn player_slot_selection(store: &WorldStore, slot: i32) -> i32 {
    store
        .inventory()
        .player_slots
        .iter()
        .find(|state| state.slot == slot)
        .map(|state| state.local_selected_bundle_item_index)
        .unwrap()
}

pub(super) fn player_slot_item(store: &WorldStore, slot: i32) -> ProtocolItemStackSummary {
    store
        .inventory()
        .player_slots
        .iter()
        .find(|state| state.slot == slot)
        .map(|state| state.item.clone())
        .unwrap_or_else(ProtocolItemStackSummary::empty)
}

pub(super) fn inventory_menu_slot_item(store: &WorldStore, slot: i16) -> ProtocolItemStackSummary {
    store
        .inventory()
        .inventory_menu_slot_item(slot)
        .cloned()
        .unwrap_or_else(ProtocolItemStackSummary::empty)
}

pub(super) fn open_container_slots(store: &WorldStore) -> Vec<ContainerSlot> {
    let inventory = store.inventory();
    inventory
        .container_slots(inventory.open_container.as_ref().unwrap())
        .map(ContainerSlotRef::to_owned)
        .collect()
}

pub(super) fn open_container_slot_item(store: &WorldStore, slot: i16) -> ProtocolItemStackSummary {
    let inventory = store.inventory();
    inventory
        .container_slot(inventory.open_container.as_ref().unwrap(), slot)
        .map(|row| row.item.clone())
        .unwrap_or_else(ProtocolItemStackSummary::empty)
}

pub(super) fn container_slot_selection(store: &WorldStore, slot: i16) -> i32 {
    let inventory = store.inventory();
    inventory
        .container_slot(inventory.open_container.as_ref().unwrap(), slot)
        .unwrap()
        .local_selected_bundle_item_index
}

pub(super) fn merchant_offers(container_id: i32, offer_count: usize) -> ProtocolMerchantOffers {
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

pub(super) fn item_cost(item_id: i32, count: i32) -> ProtocolItemCostSummary {
    ProtocolItemCostSummary {
        item_id,
        count,
        component_predicate: Default::default(),
    }
}
