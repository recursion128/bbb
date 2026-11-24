use super::*;

pub(super) fn local_player_pose(
    position: [f64; 3],
    y_rot: f32,
    x_rot: f32,
) -> LocalPlayerPoseState {
    LocalPlayerPoseState {
        position: bbb_protocol::packets::Vec3d {
            x: position[0],
            y: position[1],
            z: position[2],
        },
        y_rot,
        x_rot,
        ..LocalPlayerPoseState::default()
    }
}

pub(super) fn destroy_stage_test_textures() -> TerrainTextureState {
    let images = (0..10)
        .map(|stage| {
            bbb_client_assets::SpriteImage::new(
                format!("minecraft:block/destroy_stage_{stage}"),
                1,
                1,
                vec![255, 255, 255, 255],
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let atlas = bbb_client_assets::AtlasPacker::new(16, 1)
        .unwrap()
        .stitch(&images)
        .unwrap();
    TerrainTextureState::from_layout(&atlas.layout, None, None, None)
}

pub(super) fn test_add_entity(id: i32, entity_type_id: i32) -> bbb_protocol::packets::AddEntity {
    bbb_protocol::packets::AddEntity {
        id,
        uuid: uuid::Uuid::from_u128(id as u128),
        entity_type_id,
        position: bbb_protocol::packets::Vec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        delta_movement: bbb_protocol::packets::Vec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    }
}

pub(super) fn add_remote_supporting_block_test_entity(
    world: &mut WorldStore,
    entity_id: i32,
    block_x: i32,
    entity_y: f64,
    block_state_id: i32,
) {
    let mut packet = test_add_entity(
        entity_id,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
    );
    packet.position = bbb_protocol::packets::Vec3d {
        x: f64::from(block_x) + 0.5,
        y: entity_y,
        z: 0.5,
    };
    world.apply_add_entity(packet);
    set_lightmap_test_block(
        world,
        BlockPos {
            x: block_x,
            y: 0,
            z: 0,
        },
        block_state_id,
    );
    assert!(
        world.apply_entity_position_sync(bbb_protocol::packets::EntityPositionSync {
            id: entity_id,
            position: bbb_protocol::packets::Vec3d {
                x: f64::from(block_x) + 0.5,
                y: entity_y,
                z: 0.5,
            },
            delta_movement: bbb_protocol::packets::Vec3d::default(),
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: true,
        })
    );
}

pub(super) fn test_bool_data(data_id: u8, value: bool) -> bbb_protocol::packets::EntityDataValue {
    bbb_protocol::packets::EntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        value: bbb_protocol::packets::EntityDataValueKind::Boolean(value),
    }
}

pub(super) fn test_byte_data(data_id: u8, value: i8) -> bbb_protocol::packets::EntityDataValue {
    bbb_protocol::packets::EntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        value: bbb_protocol::packets::EntityDataValueKind::Byte(value),
    }
}

pub(super) fn test_humanoid_arm_data(
    data_id: u8,
    arm_id: i32,
) -> bbb_protocol::packets::EntityDataValue {
    bbb_protocol::packets::EntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_HUMANOID_ARM_ID,
        value: bbb_protocol::packets::EntityDataValueKind::HumanoidArm(arm_id),
    }
}

pub(super) fn test_item_stack_data(
    data_id: u8,
    item: bbb_protocol::packets::ItemStackSummary,
) -> bbb_protocol::packets::EntityDataValue {
    bbb_protocol::packets::EntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
        value: bbb_protocol::packets::EntityDataValueKind::ItemStack(item),
    }
}

pub(super) fn record_trim_material_registry(world: &mut WorldStore) {
    world.record_registry_data(bbb_protocol::packets::RegistryData {
        registry: "minecraft:trim_material".to_string(),
        entries: vec![
            bbb_protocol::packets::RegistryDataEntry {
                id: "minecraft:quartz".to_string(),
                raw_data: None,
            },
            bbb_protocol::packets::RegistryDataEntry {
                id: "minecraft:iron".to_string(),
                raw_data: None,
            },
        ],
        raw_payload_len: 0,
    });
}

pub(super) fn efficiency_enchantment_attribute_nbt() -> Vec<u8> {
    let mut out = vec![10];
    write_enchantment_definition_fields(&mut out, "minecraft:iron_pickaxe");
    out.push(10);
    write_enchantment_nbt_string(&mut out, "description");
    for (name, value) in [
        ("translate", "enchantment.example.synced_efficiency"),
        ("color", "#123456"),
    ] {
        out.push(8);
        write_enchantment_nbt_string(&mut out, name);
        write_enchantment_nbt_string(&mut out, value);
    }
    out.push(0);
    out.push(3);
    write_enchantment_nbt_string(&mut out, "max_level");
    out.extend(3i32.to_be_bytes());
    out.push(9);
    write_enchantment_nbt_string(&mut out, "slots");
    out.push(8);
    out.extend(1i32.to_be_bytes());
    write_enchantment_nbt_string(&mut out, "mainhand");

    out.push(10);
    write_enchantment_nbt_string(&mut out, "effects");
    out.push(9);
    write_enchantment_nbt_string(&mut out, "minecraft:attributes");
    out.push(10);
    out.extend(1i32.to_be_bytes());
    out.push(10);
    write_enchantment_nbt_string(&mut out, "amount");
    out.push(8);
    write_enchantment_nbt_string(&mut out, "type");
    write_enchantment_nbt_string(&mut out, "minecraft:levels_squared");
    out.push(5);
    write_enchantment_nbt_string(&mut out, "added");
    out.extend(1.0f32.to_be_bytes());
    out.push(0);
    for (name, value) in [
        ("attribute", "minecraft:mining_efficiency"),
        ("id", "minecraft:enchantment.efficiency"),
        ("operation", "add_value"),
    ] {
        out.push(8);
        write_enchantment_nbt_string(&mut out, name);
        write_enchantment_nbt_string(&mut out, value);
    }
    out.push(0);
    out.push(0);
    out.push(0);
    out
}

pub(super) fn crossbow_charge_time_enchantment_nbt(seconds: f32) -> Vec<u8> {
    let mut out = vec![10];
    write_enchantment_definition_fields(&mut out, "minecraft:crossbow");
    out.push(8);
    write_enchantment_nbt_string(&mut out, "description");
    write_enchantment_nbt_string(&mut out, "Patient Loading");
    out.push(3);
    write_enchantment_nbt_string(&mut out, "max_level");
    out.extend(3_i32.to_be_bytes());
    out.push(9);
    write_enchantment_nbt_string(&mut out, "slots");
    out.push(8);
    out.extend(1_i32.to_be_bytes());
    write_enchantment_nbt_string(&mut out, "mainhand");
    out.push(10);
    write_enchantment_nbt_string(&mut out, "effects");
    out.push(10);
    write_enchantment_nbt_string(&mut out, "minecraft:crossbow_charge_time");
    out.push(8);
    write_enchantment_nbt_string(&mut out, "type");
    write_enchantment_nbt_string(&mut out, "minecraft:set");
    out.push(5);
    write_enchantment_nbt_string(&mut out, "value");
    out.extend(seconds.to_be_bytes());
    out.extend([0, 0, 0]);
    out
}

fn write_enchantment_definition_fields(out: &mut Vec<u8>, supported_item: &str) {
    out.push(8);
    write_enchantment_nbt_string(out, "supported_items");
    write_enchantment_nbt_string(out, supported_item);
    for (name, value) in [("weight", 1_i32), ("anvil_cost", 0)] {
        out.push(3);
        write_enchantment_nbt_string(out, name);
        out.extend(value.to_be_bytes());
    }
    for name in ["min_cost", "max_cost"] {
        out.push(10);
        write_enchantment_nbt_string(out, name);
        for field in ["base", "per_level_above_first"] {
            out.push(3);
            write_enchantment_nbt_string(out, field);
            out.extend(0_i32.to_be_bytes());
        }
        out.push(0);
    }
}

pub(super) fn write_enchantment_nbt_string(out: &mut Vec<u8>, value: &str) {
    let bytes = value.as_bytes();
    out.extend((bytes.len() as u16).to_be_bytes());
    out.extend(bytes);
}

pub(super) fn merchant_offers(
    container_id: i32,
    offer_count: usize,
    out_of_stock_index: Option<usize>,
) -> MerchantOffers {
    MerchantOffers {
        container_id,
        offers: (0..offer_count)
            .map(|index| MerchantOffer {
                buy_a: item_cost(42 + index as i32, 3),
                sell: item_stack(99 + index as i32, 1),
                buy_b: (index % 2 == 0).then(|| item_cost(52 + index as i32, 2)),
                is_out_of_stock: out_of_stock_index == Some(index),
                uses: if out_of_stock_index == Some(index) {
                    12
                } else {
                    1
                },
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
        can_restock: true,
    }
}

pub(super) fn item_cost(item_id: i32, count: i32) -> ItemCostSummary {
    ItemCostSummary {
        item_id,
        count,
        component_predicate: Default::default(),
    }
}

pub(super) fn stonecutter_world_with_builtin_slot_displays(
    encoded_pattern_holder_id: i32,
) -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
    });
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 38];
    items[0] = item_stack(3, 1);
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.set_furnace_fuel_item_ids(BTreeSet::from([1]));
    set_stonecutter_item_tag(&mut world, vec![1]);
    world.record_registry_entries(
        TRIM_MATERIAL_REGISTRY,
        0,
        vec![RegistryPacketEntry::with_raw_data(
            "example:copper",
            stonecutter_trim_material_nbt("copper", "Copper"),
        )],
    );
    world.record_registry_entries(
        TRIM_PATTERN_REGISTRY,
        0,
        vec![RegistryPacketEntry::with_raw_data(
            "example:spiral",
            stonecutter_trim_pattern_nbt("example:spiral", "Spiral", true),
        )],
    );
    world.set_default_item_component_codec_values(BTreeMap::from([
        (
            1,
            BTreeMap::from([(VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID, serde_json::json!({}))]),
        ),
        (
            2,
            BTreeMap::from([(VANILLA_DATA_COMPONENT_DYE_TYPE_ID, serde_json::json!("red"))]),
        ),
        (
            3,
            BTreeMap::from([(
                VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
                serde_json::json!("example:copper"),
            )]),
        ),
    ]));
    world.begin_registry_configuration();
    world.finish_registry_configuration().unwrap();
    let packet = stonecutter_builtin_update_recipes(encoded_pattern_holder_id);
    for recipe in &packet.stonecutter_recipes {
        world
            .resolve_slot_display_first_stack(&recipe.option_display)
            .unwrap();
    }
    world.apply_update_recipes(packet);
    assert_eq!(world.recipes().stonecutter_recipes.len(), 11);
    world
}

pub(super) fn stonecutter_builtin_update_recipes(
    encoded_pattern_holder_id: i32,
) -> bbb_protocol::packets::UpdateRecipes {
    let item = stonecutter_item_display(1);
    let item_stack = stonecutter_item_stack_display(1, 3);
    let displays = vec![
        slot_display_type(VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID),
        slot_display_type(VANILLA_SLOT_DISPLAY_TYPE_ANY_FUEL_ID),
        slot_display_unary(VANILLA_SLOT_DISPLAY_TYPE_WITH_ANY_POTION_ID, item.clone()),
        slot_display_only_with_component(item.clone(), VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID),
        item.clone(),
        item_stack,
        slot_display_tag("example:choices"),
        slot_display_binary(
            VANILLA_SLOT_DISPLAY_TYPE_DYED_ID,
            stonecutter_item_display(2),
            item.clone(),
        ),
        slot_display_smithing_trim(
            item.clone(),
            stonecutter_item_display(3),
            encoded_pattern_holder_id,
        ),
        slot_display_with_remainder(item.clone(), stonecutter_item_display(2)),
        slot_display_composite(vec![
            slot_display_type(VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID),
            item,
        ]),
    ];
    bbb_protocol::packets::UpdateRecipes {
        property_sets: Vec::new(),
        stonecutter_recipes: displays
            .into_iter()
            .map(
                |option_display| bbb_protocol::packets::StonecutterSelectableRecipeSummary {
                    input: bbb_protocol::packets::IngredientSummary {
                        tag: None,
                        item_ids: vec![3],
                    },
                    option_display,
                },
            )
            .collect(),
    }
}

pub(super) fn stonecutter_recipe_item_position(position: i32) -> (i32, i32) {
    (
        STONECUTTER_RECIPE_BUTTON_X
            + position % STONECUTTER_RECIPE_BUTTON_COLUMNS * STONECUTTER_RECIPE_BUTTON_WIDTH,
        STONECUTTER_RECIPE_ITEM_Y
            + position / STONECUTTER_RECIPE_BUTTON_COLUMNS * STONECUTTER_RECIPE_BUTTON_HEIGHT,
    )
}

pub(super) fn stonecutter_recipe_tooltip_at(
    world: &WorldStore,
    item_runtime: &NativeItemRuntime,
    cursor_position: (i32, i32),
) -> HudInventoryTooltip {
    hud_inventory_screen_with_local_state(
        world,
        Some(item_runtime),
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            cursor_position: Some(cursor_position),
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap()
    .tooltip
    .unwrap()
}

pub(super) fn set_stonecutter_item_tag(world: &mut WorldStore, entries: Vec<i32>) {
    world.apply_update_tags(bbb_protocol::packets::UpdateTags {
        registries: vec![bbb_protocol::packets::RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![bbb_protocol::packets::TagNetworkPayload {
                tag: "example:choices".to_string(),
                entries,
            }],
        }],
    });
}

pub(super) fn clear_stonecutter_item_tags(world: &mut WorldStore) {
    world.apply_update_tags(bbb_protocol::packets::UpdateTags {
        registries: vec![bbb_protocol::packets::RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: Vec::new(),
        }],
    });
}

pub(super) fn slot_display_type(display_type_id: i32) -> bbb_protocol::packets::SlotDisplaySummary {
    let mut payload = bbb_protocol::codec::Encoder::new();
    payload.write_var_i32(display_type_id);
    slot_display_summary(display_type_id, payload.into_inner())
}

pub(super) fn slot_display_unary(
    display_type_id: i32,
    display: bbb_protocol::packets::SlotDisplaySummary,
) -> bbb_protocol::packets::SlotDisplaySummary {
    let mut payload = bbb_protocol::codec::Encoder::new();
    payload.write_var_i32(display_type_id);
    payload.write_bytes(&display.raw_payload);
    slot_display_summary(display_type_id, payload.into_inner())
}

pub(super) fn slot_display_binary(
    display_type_id: i32,
    first: bbb_protocol::packets::SlotDisplaySummary,
    second: bbb_protocol::packets::SlotDisplaySummary,
) -> bbb_protocol::packets::SlotDisplaySummary {
    let mut payload = bbb_protocol::codec::Encoder::new();
    payload.write_var_i32(display_type_id);
    payload.write_bytes(&first.raw_payload);
    payload.write_bytes(&second.raw_payload);
    slot_display_summary(display_type_id, payload.into_inner())
}

pub(super) fn slot_display_only_with_component(
    source: bbb_protocol::packets::SlotDisplaySummary,
    component_type_id: i32,
) -> bbb_protocol::packets::SlotDisplaySummary {
    let mut payload = bbb_protocol::codec::Encoder::new();
    payload.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_ONLY_WITH_COMPONENT_ID);
    payload.write_bytes(&source.raw_payload);
    payload.write_var_i32(component_type_id);
    slot_display_summary(
        VANILLA_SLOT_DISPLAY_TYPE_ONLY_WITH_COMPONENT_ID,
        payload.into_inner(),
    )
}

pub(super) fn slot_display_smithing_trim(
    base: bbb_protocol::packets::SlotDisplaySummary,
    material: bbb_protocol::packets::SlotDisplaySummary,
    encoded_pattern_holder_id: i32,
) -> bbb_protocol::packets::SlotDisplaySummary {
    let mut payload = bbb_protocol::codec::Encoder::new();
    payload.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_SMITHING_TRIM_ID);
    payload.write_bytes(&base.raw_payload);
    payload.write_bytes(&material.raw_payload);
    payload.write_var_i32(encoded_pattern_holder_id);
    slot_display_summary(
        VANILLA_SLOT_DISPLAY_TYPE_SMITHING_TRIM_ID,
        payload.into_inner(),
    )
}

pub(super) fn slot_display_summary(
    display_type_id: i32,
    raw_payload: Vec<u8>,
) -> bbb_protocol::packets::SlotDisplaySummary {
    bbb_protocol::packets::SlotDisplaySummary {
        display_type_id,
        raw_payload,
        item_stack: None,
        tag: None,
    }
}

pub(super) fn stonecutter_recipe_display(
    input_item_id: i32,
    result_item_id: i32,
    result_count: i32,
) -> bbb_protocol::packets::StonecutterSelectableRecipeSummary {
    bbb_protocol::packets::StonecutterSelectableRecipeSummary {
        input: bbb_protocol::packets::IngredientSummary {
            tag: None,
            item_ids: vec![input_item_id],
        },
        option_display: stonecutter_item_stack_display(result_item_id, result_count),
    }
}

pub(super) fn stonecutter_item_display(item_id: i32) -> bbb_protocol::packets::SlotDisplaySummary {
    let mut raw_payload = Vec::new();
    bbb_protocol::codec::write_var_i32_to(&mut raw_payload, 4);
    bbb_protocol::codec::write_var_i32_to(&mut raw_payload, item_id);
    bbb_protocol::packets::SlotDisplaySummary {
        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,
        raw_payload,
        item_stack: Some(item_stack(item_id, 1)),
        tag: None,
    }
}

pub(super) fn slot_display_empty() -> bbb_protocol::packets::SlotDisplaySummary {
    let mut raw_payload = bbb_protocol::codec::Encoder::new();
    raw_payload.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID);
    bbb_protocol::packets::SlotDisplaySummary {
        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
        raw_payload: raw_payload.into_inner(),
        item_stack: None,
        tag: None,
    }
}

pub(super) fn stonecutter_item_stack_display(
    item_id: i32,
    count: i32,
) -> bbb_protocol::packets::SlotDisplaySummary {
    let mut raw_payload = Vec::new();
    bbb_protocol::codec::write_var_i32_to(&mut raw_payload, 5);
    bbb_protocol::codec::write_var_i32_to(&mut raw_payload, item_id);
    bbb_protocol::codec::write_var_i32_to(&mut raw_payload, count);
    bbb_protocol::codec::write_var_i32_to(&mut raw_payload, 0);
    bbb_protocol::codec::write_var_i32_to(&mut raw_payload, 0);
    bbb_protocol::packets::SlotDisplaySummary {
        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID,
        raw_payload,
        item_stack: Some(item_stack(item_id, count)),
        tag: None,
    }
}

pub(super) fn slot_display_tag(tag: &str) -> bbb_protocol::packets::SlotDisplaySummary {
    let mut raw_payload = bbb_protocol::codec::Encoder::new();
    raw_payload.write_var_i32(6);
    raw_payload.write_string(tag);
    bbb_protocol::packets::SlotDisplaySummary {
        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_TAG_ID,
        raw_payload: raw_payload.into_inner(),
        item_stack: None,
        tag: Some(tag.to_string()),
    }
}

pub(super) fn slot_display_composite(
    contents: Vec<bbb_protocol::packets::SlotDisplaySummary>,
) -> bbb_protocol::packets::SlotDisplaySummary {
    let mut raw_payload = bbb_protocol::codec::Encoder::new();
    raw_payload.write_var_i32(10);
    raw_payload.write_var_i32(i32::try_from(contents.len()).unwrap());
    for content in &contents {
        raw_payload.write_bytes(&content.raw_payload);
    }
    bbb_protocol::packets::SlotDisplaySummary {
        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_COMPOSITE_ID,
        raw_payload: raw_payload.into_inner(),
        item_stack: None,
        tag: None,
    }
}

pub(super) fn slot_display_with_remainder(
    input: bbb_protocol::packets::SlotDisplaySummary,
    remainder: bbb_protocol::packets::SlotDisplaySummary,
) -> bbb_protocol::packets::SlotDisplaySummary {
    let mut raw_payload = bbb_protocol::codec::Encoder::new();
    raw_payload.write_var_i32(9);
    raw_payload.write_bytes(&input.raw_payload);
    raw_payload.write_bytes(&remainder.raw_payload);
    bbb_protocol::packets::SlotDisplaySummary {
        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_WITH_REMAINDER_ID,
        raw_payload: raw_payload.into_inner(),
        item_stack: None,
        tag: None,
    }
}

pub(super) fn cartography_table_world_with_items(
    items: Vec<bbb_protocol::packets::ItemStackSummary>,
) -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
        title: bbb_protocol::RichTextComponent::plain("Cartography Table"),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world
}

pub(super) fn apply_cartography_map_state(
    world: &mut WorldStore,
    map_id: i32,
    scale: i8,
    locked: bool,
) {
    assert!(
        world.apply_map_item_data(bbb_protocol::packets::MapItemData {
            map_id,
            scale,
            locked,
            decorations: None,
            color_patch: None,
        })
    );
}

pub(super) fn map_stack(
    item_id: i32,
    count: i32,
    map_id: i32,
    post_processing: Option<bbb_protocol::packets::MapPostProcessingSummary>,
) -> bbb_protocol::packets::ItemStackSummary {
    let mut item = item_stack(item_id, count);
    item.component_patch.map_id = Some(map_id);
    item.component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID);
    if let Some(post_processing) = post_processing {
        item.component_patch.map_post_processing = Some(post_processing);
        item.component_patch.added_type_ids.push(48);
    }
    item
}

pub(super) fn item_stack(item_id: i32, count: i32) -> bbb_protocol::packets::ItemStackSummary {
    bbb_protocol::packets::ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    }
}

pub(super) fn open_recipe_book_crafting_table_world() -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 46],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_recipe_book_settings(bbb_protocol::packets::RecipeBookSettings {
        crafting: bbb_protocol::packets::RecipeBookTypeSettings {
            open: true,
            filtering: false,
        },
        furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        blast_furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        smoker: bbb_protocol::packets::RecipeBookTypeSettings::default(),
    });
    world
}

pub(super) fn open_recipe_book_furnace_world() -> WorldStore {
    open_recipe_book_furnace_family_world(VANILLA_MENU_TYPE_FURNACE_ID)
}

pub(super) fn open_recipe_book_furnace_family_world(menu_type_id: i32) -> WorldStore {
    let mut world = WorldStore::new();
    let title = match menu_type_id {
        VANILLA_MENU_TYPE_FURNACE_ID => "Furnace",
        VANILLA_MENU_TYPE_BLAST_FURNACE_ID => "Blast Furnace",
        VANILLA_MENU_TYPE_SMOKER_ID => "Smoker",
        _ => panic!("unsupported furnace-family menu type {menu_type_id}"),
    };
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id,
        title: bbb_protocol::RichTextComponent::plain(title),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 39],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    let open = bbb_protocol::packets::RecipeBookTypeSettings {
        open: true,
        filtering: false,
    };
    let mut settings = bbb_protocol::packets::RecipeBookSettings::default();
    match menu_type_id {
        VANILLA_MENU_TYPE_FURNACE_ID => settings.furnace = open,
        VANILLA_MENU_TYPE_BLAST_FURNACE_ID => settings.blast_furnace = open,
        VANILLA_MENU_TYPE_SMOKER_ID => settings.smoker = open,
        _ => unreachable!("menu type validated above"),
    }
    world.apply_recipe_book_settings(settings);
    world
}

pub(super) fn recipe_book_ghost_item_runtime() -> NativeItemRuntime {
    let root = unique_runtime_temp_dir("recipe-book-ghost-items");
    let assets = runtime_assets_dir(&root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    for (model_id, rgba) in [
        ("crafting_table", [120, 80, 40, 255]),
        ("stick", [150, 95, 45, 255]),
        ("oak_planks", [190, 145, 80, 255]),
        ("iron_axe", [180, 180, 190, 255]),
        ("golden_sword", [240, 210, 80, 255]),
    ] {
        write_runtime_json(
            &assets.join("items").join(format!("{model_id}.json")),
            &format!(
                r#"{{
                    "model": {{
                        "type": "minecraft:model",
                        "model": "minecraft:item/{model_id}"
                    }}
                }}"#
            ),
        );
        write_flat_runtime_item_model_and_texture(&assets, model_id, &rgba);
    }
    write_runtime_json(
        &assets.join("lang").join("en_us.json"),
        r#"{
            "item.minecraft.crafting_table": "Crafting Table",
            "item.minecraft.stick": "Stick",
            "item.minecraft.oak_planks": "Wooden Boards",
            "item.minecraft.iron_axe": "Iron Axe",
            "item.minecraft.golden_sword": "Golden Sword"
        }"#,
    );
    crate::item_registry_test_fixture::write_item_registry_fixture(
        root.as_ref(),
        &[
            "crafting_table",
            "stick",
            "oak_planks",
            "iron_axe",
            "golden_sword",
        ],
    );
    let runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    std::fs::remove_dir_all(root).unwrap();
    runtime
}

pub(super) fn crafting_recipe_book_search_recipe_indices(
    world: &WorldStore,
    item_runtime: &NativeItemRuntime,
    search_text: &str,
) -> Vec<i32> {
    crafting_recipe_book_collections(
        world,
        RecipeBookCraftingGrid {
            width: 3,
            height: 3,
        },
        1,
        false,
        search_text,
        Some(item_runtime),
    )
    .iter()
    .filter_map(|collection| {
        collection
            .recipe_index_and_craftable_at_slot_select_index(0)
            .map(|(recipe_index, _)| recipe_index)
    })
    .collect()
}

pub(super) fn shapeless_crafting_recipe_book_entry(
    id: i32,
    category_id: i32,
    group: Option<i32>,
    result_item_id: i32,
) -> bbb_protocol::packets::RecipeBookAddEntry {
    shapeless_crafting_recipe_book_entry_with_requirements(
        id,
        category_id,
        group,
        result_item_id,
        Vec::new(),
    )
}

pub(super) fn shapeless_crafting_recipe_book_entry_with_display_ingredients(
    id: i32,
    category_id: i32,
    group: Option<i32>,
    result_item_id: i32,
    display_ingredients: Vec<bbb_protocol::packets::SlotDisplaySummary>,
) -> bbb_protocol::packets::RecipeBookAddEntry {
    let mut entry = shapeless_crafting_recipe_book_entry(id, category_id, group, result_item_id);
    if let Some(bbb_protocol::packets::CraftingRecipeDisplaySummary::Shapeless {
        ingredients,
        ..
    }) = entry.contents.display.crafting.as_deref_mut()
    {
        *ingredients = display_ingredients;
    }
    entry
}

pub(super) fn shapeless_crafting_recipe_book_entry_with_requirements(
    id: i32,
    category_id: i32,
    group: Option<i32>,
    result_item_id: i32,
    requirements: Vec<Vec<i32>>,
) -> bbb_protocol::packets::RecipeBookAddEntry {
    shapeless_crafting_recipe_book_entry_with_requirement_summaries(
        id,
        category_id,
        group,
        result_item_id,
        requirements
            .into_iter()
            .map(|item_ids| bbb_protocol::packets::IngredientSummary {
                tag: None,
                item_ids,
            })
            .collect(),
    )
}

pub(super) fn shapeless_crafting_recipe_book_entry_with_requirement_summaries(
    id: i32,
    category_id: i32,
    group: Option<i32>,
    result_item_id: i32,
    requirements: Vec<bbb_protocol::packets::IngredientSummary>,
) -> bbb_protocol::packets::RecipeBookAddEntry {
    bbb_protocol::packets::RecipeBookAddEntry {
        contents: bbb_protocol::packets::RecipeDisplayEntry {
            id: bbb_protocol::packets::RecipeDisplayId { index: id },
            display: bbb_protocol::packets::RecipeDisplaySummary {
                display_type: bbb_protocol::packets::RecipeDisplayType::CraftingShapeless,
                raw_body: Vec::new(),
                crafting: Some(Box::new(
                    bbb_protocol::packets::CraftingRecipeDisplaySummary::Shapeless {
                        ingredients: Vec::new(),
                        result: bbb_protocol::packets::SlotDisplaySummary {
                            display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID,
                            raw_payload: Vec::new(),
                            item_stack: Some(item_stack(result_item_id, 1)),
                            tag: None,
                        },
                        crafting_station: bbb_protocol::packets::SlotDisplaySummary {
                            display_type_id: VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
                            raw_payload: Vec::new(),
                            item_stack: None,
                            tag: None,
                        },
                    },
                )),
                furnace: None,
                stonecutter: None,
                smithing: None,
            },
            group,
            category_id,
            crafting_requirements: (!requirements.is_empty()).then_some(requirements),
        },
        flags: 0,
        notification: false,
        highlight: false,
    }
}

pub(super) fn furnace_recipe_book_entry(
    id: i32,
    category_id: i32,
    group: Option<i32>,
    result_item_id: i32,
) -> bbb_protocol::packets::RecipeBookAddEntry {
    furnace_recipe_book_entry_with_requirements(id, category_id, group, result_item_id, Vec::new())
}

pub(super) fn furnace_recipe_book_entry_with_requirements(
    id: i32,
    category_id: i32,
    group: Option<i32>,
    result_item_id: i32,
    requirements: Vec<Vec<i32>>,
) -> bbb_protocol::packets::RecipeBookAddEntry {
    bbb_protocol::packets::RecipeBookAddEntry {
        contents: bbb_protocol::packets::RecipeDisplayEntry {
            id: bbb_protocol::packets::RecipeDisplayId { index: id },
            display: bbb_protocol::packets::RecipeDisplaySummary {
                display_type: bbb_protocol::packets::RecipeDisplayType::Furnace,
                raw_body: Vec::new(),
                crafting: None,
                furnace: Some(Box::new(furnace_recipe_display_body(2, 1, result_item_id))),
                stonecutter: None,
                smithing: None,
            },
            group,
            category_id,
            crafting_requirements: (!requirements.is_empty()).then(|| {
                requirements
                    .into_iter()
                    .map(|item_ids| bbb_protocol::packets::IngredientSummary {
                        tag: None,
                        item_ids,
                    })
                    .collect()
            }),
        },
        flags: 0,
        notification: false,
        highlight: false,
    }
}

pub(super) fn furnace_recipe_display(
    ingredient_item_id: i32,
    fuel_item_id: i32,
    result_item_id: i32,
) -> bbb_protocol::packets::RecipeDisplaySummary {
    bbb_protocol::packets::RecipeDisplaySummary {
        display_type: bbb_protocol::packets::RecipeDisplayType::Furnace,
        raw_body: Vec::new(),
        crafting: None,
        furnace: Some(Box::new(furnace_recipe_display_body(
            ingredient_item_id,
            fuel_item_id,
            result_item_id,
        ))),
        stonecutter: None,
        smithing: None,
    }
}

pub(super) fn furnace_recipe_display_body(
    ingredient_item_id: i32,
    fuel_item_id: i32,
    result_item_id: i32,
) -> bbb_protocol::packets::FurnaceRecipeDisplaySummary {
    bbb_protocol::packets::FurnaceRecipeDisplaySummary {
        ingredient: stonecutter_item_display(ingredient_item_id),
        fuel: stonecutter_item_display(fuel_item_id),
        result: stonecutter_item_stack_display(result_item_id, 1),
        crafting_station: bbb_protocol::packets::SlotDisplaySummary {
            display_type_id: VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
            raw_payload: vec![0],
            item_stack: None,
            tag: None,
        },
        duration: 200,
        experience_bits: 0.0_f32.to_bits(),
    }
}

pub(super) fn apply_item_tags(world: &mut WorldStore, tags: Vec<(&str, Vec<i32>)>) {
    world.apply_update_tags(bbb_protocol::packets::UpdateTags {
        registries: vec![bbb_protocol::packets::RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: tags
                .into_iter()
                .map(|(tag, entries)| bbb_protocol::packets::TagNetworkPayload {
                    tag: tag.to_string(),
                    entries,
                })
                .collect(),
        }],
    });
}

pub(super) fn apply_block_tags(world: &mut WorldStore, tags: Vec<(&str, Vec<i32>)>) {
    world.apply_update_tags(bbb_protocol::packets::UpdateTags {
        registries: vec![bbb_protocol::packets::RegistryTags {
            registry: "minecraft:block".to_string(),
            tags: tags
                .into_iter()
                .map(|(tag, entries)| bbb_protocol::packets::TagNetworkPayload {
                    tag: tag.to_string(),
                    entries,
                })
                .collect(),
        }],
    });
}

pub(super) fn apply_fluid_tags(world: &mut WorldStore, tags: Vec<(&str, Vec<i32>)>) {
    world.apply_update_tags(bbb_protocol::packets::UpdateTags {
        registries: vec![bbb_protocol::packets::RegistryTags {
            registry: "minecraft:fluid".to_string(),
            tags: tags
                .into_iter()
                .map(|(tag, entries)| bbb_protocol::packets::TagNetworkPayload {
                    tag: tag.to_string(),
                    entries,
                })
                .collect(),
        }],
    });
}

pub(super) fn apply_entity_type_tags(world: &mut WorldStore, tags: Vec<(&str, Vec<i32>)>) {
    world.apply_update_tags(bbb_protocol::packets::UpdateTags {
        registries: vec![bbb_protocol::packets::RegistryTags {
            registry: "minecraft:entity_type".to_string(),
            tags: tags
                .into_iter()
                .map(|(tag, entries)| bbb_protocol::packets::TagNetworkPayload {
                    tag: tag.to_string(),
                    entries,
                })
                .collect(),
        }],
    });
}

pub(super) fn written_book_stack(pages: Vec<&str>) -> bbb_protocol::packets::ItemStackSummary {
    let mut item = item_stack(42, 1);
    let pages: Vec<String> = pages.into_iter().map(str::to_string).collect();
    let page_filters = vec![None; pages.len()];
    item.component_patch.written_book = Some(WrittenBookContentSummary {
        title: "Guide".into(),
        title_filter: None,
        author: "Alex".into(),
        generation: 0,
        pages,
        page_filters,
        resolved: true,
    });
    item
}

pub(super) fn item_stack_with_damage(
    item_id: i32,
    count: i32,
    max_damage: i32,
    damage: i32,
    unbreakable: bool,
) -> bbb_protocol::packets::ItemStackSummary {
    let mut item = item_stack(item_id, count);
    item.component_patch.max_damage = Some(max_damage);
    item.component_patch.damage = Some(damage);
    item.component_patch.unbreakable = unbreakable;
    item.component_patch.added_type_ids.extend([
        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
    ]);
    if unbreakable {
        item.component_patch
            .added_type_ids
            .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID);
    }
    item
}

pub(super) fn write_stonecutter_slot_display_item_assets(root: &Path) {
    write_runtime_tooltip_item_assets(root);
    let assets = runtime_assets_dir(root);
    for (item, color) in [
        ("test_sword", [180, 180, 190, 255]),
        ("command_block", [180, 140, 100, 255]),
    ] {
        write_runtime_json(
            &assets.join("items").join(format!("{item}.json")),
            &format!(
                r#"{{
                    "model": {{
                        "type": "minecraft:model",
                        "model": "minecraft:item/{item}"
                    }}
                }}"#
            ),
        );
        write_flat_runtime_item_model_and_texture(&assets, item, &color);
    }
}

pub(super) fn stonecutter_trim_material_nbt(asset_name: &str, description: &str) -> Vec<u8> {
    let mut out = vec![10];
    write_stonecutter_nbt_string_field(&mut out, "asset_name", asset_name);
    write_stonecutter_trim_description(&mut out, description);
    out.push(0);
    out
}

pub(super) fn stonecutter_trim_pattern_nbt(
    asset_id: &str,
    description: &str,
    decal: bool,
) -> Vec<u8> {
    let mut out = vec![10];
    write_stonecutter_nbt_string_field(&mut out, "asset_id", asset_id);
    write_stonecutter_trim_description(&mut out, description);
    out.push(1);
    write_stonecutter_nbt_string(&mut out, "decal");
    out.push(u8::from(decal));
    out.push(0);
    out
}

pub(super) fn write_stonecutter_trim_description(out: &mut Vec<u8>, text: &str) {
    out.push(10);
    write_stonecutter_nbt_string(out, "description");
    write_stonecutter_nbt_string_field(out, "text", text);
    out.push(0);
}

pub(super) fn write_stonecutter_nbt_string_field(out: &mut Vec<u8>, name: &str, value: &str) {
    out.push(8);
    write_stonecutter_nbt_string(out, name);
    write_stonecutter_nbt_string(out, value);
}

pub(super) fn write_stonecutter_nbt_string(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u16).to_be_bytes());
    out.extend_from_slice(value.as_bytes());
}

pub(super) fn write_runtime_tooltip_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "block/",
                    "source": "block"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("test_combo.json"),
        r#"{
            "model": {
                "type": "minecraft:model",
                "model": "minecraft:item/test_combo"
            }
        }"#,
    );
    write_runtime_json(
        &assets.join("models").join("item").join("test_combo.json"),
        r#"{
            "textures": {
                "layer0": "minecraft:item/test_combo"
            }
        }"#,
    );
    write_runtime_json(
        &assets.join("lang").join("en_us.json"),
        r#"{
            "item.minecraft.test_combo": "Test Combo",
            "item.minecraft.test_sword": "Test Sword",
            "block.minecraft.command_block": "Command Block",
            "item.disabled": "Disabled item",
            "item.durability": "Durability: %s / %s",
            "item.components": "%s component(s)",
            "enchantment.minecraft.efficiency": "Efficiency",
            "enchantment.example.synced_efficiency": "Synced Efficiency",
            "enchantment.level.1": "I",
            "enchantment.level.3": "III",
            "attribute.modifier.plus.0": "+%s %s",
            "attribute.modifier.equals.0": "%s %s",
            "attribute.name.attack_damage": "Attack Damage",
            "attribute.name.attack_speed": "Attack Speed",
            "attribute.name.mining_efficiency": "Mining Efficiency",
            "item.modifiers.mainhand": "When in Main Hand:",
            "item.op_warning.line1": "Operator data",
            "item.op_warning.line2": "Can alter commands",
            "item.op_warning.line3": "Use with care"
        }"#,
    );
    write_runtime_png(
        &assets.join("textures").join("item").join("test_combo.png"),
        &[80, 120, 160, 255],
    );
    crate::item_registry_test_fixture::write_item_registry_fixture_with_profiles(
        root,
        &["test_combo", "test_sword", "command_block"],
        &[
            ("command_block", "command_block"),
            ("test_sword", "iron_sword"),
        ],
    );
}

pub(super) fn write_runtime_beacon_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    for (item_id, rgba) in [
        ("netherite_ingot", [40, 50, 60, 255]),
        ("emerald", [40, 200, 80, 255]),
        ("diamond", [80, 220, 240, 255]),
        ("gold_ingot", [240, 190, 40, 255]),
        ("iron_ingot", [210, 210, 220, 255]),
    ] {
        write_runtime_json(
            &assets.join("items").join(format!("{item_id}.json")),
            &format!(
                r#"{{
                    "model": {{
                        "type": "minecraft:model",
                        "model": "minecraft:item/{item_id}"
                    }}
                }}"#
            ),
        );
        write_flat_runtime_item_model_and_texture(&assets, item_id, &rgba);
    }
    write_runtime_json(
        &assets.join("lang").join("en_us.json"),
        r#"{
            "block.minecraft.beacon.primary": "Main Power",
            "block.minecraft.beacon.secondary": "Auxiliary Power",
            "effect.minecraft.speed": "Swift",
            "effect.minecraft.strength": "Might",
            "gui.done": "Apply",
            "gui.cancel": "Abort"
        }"#,
    );
    crate::item_registry_test_fixture::write_item_registry_fixture(
        root,
        &[
            "netherite_ingot",
            "emerald",
            "diamond",
            "gold_ingot",
            "iron_ingot",
        ],
    );
}

pub(super) fn write_runtime_bow_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("bow.json"),
        r#"{
            "model": {
                "type": "minecraft:condition",
                "property": "minecraft:using_item",
                "on_false": {
                    "type": "minecraft:model",
                    "model": "minecraft:item/bow"
                },
                "on_true": {
                    "type": "minecraft:range_dispatch",
                    "property": "minecraft:use_duration",
                    "scale": 0.05,
                    "entries": [
                        {
                            "threshold": 0.65,
                            "model": {
                                "type": "minecraft:model",
                                "model": "minecraft:item/bow_pulling_1"
                            }
                        },
                        {
                            "threshold": 0.9,
                            "model": {
                                "type": "minecraft:model",
                                "model": "minecraft:item/bow_pulling_2"
                            }
                        }
                    ],
                    "fallback": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/bow_pulling_0"
                    }
                }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(&assets, "bow", &[80, 120, 160, 255]);
    write_flat_runtime_item_model_and_texture(&assets, "bow_pulling_0", &[160, 80, 120, 255]);
    write_flat_runtime_item_model_and_texture(&assets, "bow_pulling_1", &[120, 160, 80, 255]);
    write_flat_runtime_item_model_and_texture(&assets, "bow_pulling_2", &[160, 120, 80, 255]);
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(root, &["bow"]);
}

pub(super) fn write_runtime_team_tint_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{ "sources": [] }"#,
    );
    write_runtime_json(
        &assets.join("items").join("team_tint.json"),
        r#"{
            "model": {
                "type": "minecraft:model",
                "model": "minecraft:item/team_tint",
                "tints": [
                    { "type": "minecraft:team", "default": 1193046 }
                ]
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(&assets, "team_tint", &[255, 255, 255, 255]);
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(root, &["team_tint"]);
}

pub(super) fn write_runtime_crossbow_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("crossbow.json"),
        r#"{
            "model": {
                "type": "minecraft:select",
                "property": "minecraft:charge_type",
                "cases": [],
                "fallback": {
                    "type": "minecraft:condition",
                    "property": "minecraft:using_item",
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/crossbow"
                    },
                    "on_true": {
                        "type": "minecraft:range_dispatch",
                        "property": "minecraft:crossbow/pull",
                        "entries": [
                            {
                                "threshold": 0.58,
                                "model": {
                                    "type": "minecraft:model",
                                    "model": "minecraft:item/crossbow_pulling_1"
                                }
                            },
                            {
                                "threshold": 1.0,
                                "model": {
                                    "type": "minecraft:model",
                                    "model": "minecraft:item/crossbow_pulling_2"
                                }
                            }
                        ],
                        "fallback": {
                            "type": "minecraft:model",
                            "model": "minecraft:item/crossbow_pulling_0"
                        }
                    }
                }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(&assets, "crossbow", &[40, 80, 120, 255]);
    write_flat_runtime_item_model_and_texture(&assets, "crossbow_pulling_0", &[70, 100, 130, 255]);
    write_flat_runtime_item_model_and_texture(&assets, "crossbow_pulling_1", &[100, 130, 70, 255]);
    write_flat_runtime_item_model_and_texture(&assets, "crossbow_pulling_2", &[130, 70, 100, 255]);
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(root, &["crossbow"]);
}

pub(super) fn write_runtime_main_hand_select_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("hand_selector.json"),
        r#"{
            "model": {
                "type": "minecraft:select",
                "property": "minecraft:main_hand",
                "cases": [
                    {
                        "when": "left",
                        "model": { "type": "minecraft:model", "model": "minecraft:item/hand_selector_left" }
                    },
                    {
                        "when": "right",
                        "model": { "type": "minecraft:model", "model": "minecraft:item/hand_selector_right" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/hand_selector" }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(&assets, "hand_selector", &[40, 80, 120, 255]);
    write_flat_runtime_item_model_and_texture(&assets, "hand_selector_left", &[120, 40, 80, 255]);
    write_flat_runtime_item_model_and_texture(&assets, "hand_selector_right", &[80, 120, 40, 255]);
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(root, &["hand_selector"]);
}

pub(super) fn write_runtime_context_dimension_select_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("dimension_selector.json"),
        r#"{
            "model": {
                "type": "minecraft:select",
                "property": "minecraft:context_dimension",
                "cases": [
                    {
                        "when": "minecraft:overworld",
                        "model": { "type": "minecraft:model", "model": "minecraft:item/dimension_selector_overworld" }
                    },
                    {
                        "when": "minecraft:the_nether",
                        "model": { "type": "minecraft:model", "model": "minecraft:item/dimension_selector_nether" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/dimension_selector" }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(&assets, "dimension_selector", &[40, 80, 120, 255]);
    write_flat_runtime_item_model_and_texture(
        &assets,
        "dimension_selector_overworld",
        &[80, 120, 40, 255],
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "dimension_selector_nether",
        &[120, 40, 80, 255],
    );
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(root, &["dimension_selector"]);
}

pub(super) fn write_runtime_context_entity_type_select_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("entity_selector.json"),
        r#"{
            "model": {
                "type": "minecraft:select",
                "property": "minecraft:context_entity_type",
                "cases": [
                    {
                        "when": "minecraft:player",
                        "model": { "type": "minecraft:model", "model": "minecraft:item/entity_selector_player" }
                    },
                    {
                        "when": "minecraft:cow",
                        "model": { "type": "minecraft:model", "model": "minecraft:item/entity_selector_cow" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/entity_selector" }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(&assets, "entity_selector", &[40, 80, 120, 255]);
    write_flat_runtime_item_model_and_texture(
        &assets,
        "entity_selector_player",
        &[80, 120, 40, 255],
    );
    write_flat_runtime_item_model_and_texture(&assets, "entity_selector_cow", &[120, 40, 80, 255]);
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(root, &["entity_selector"]);
}

pub(super) fn write_runtime_view_entity_condition_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("view_entity_condition.json"),
        r#"{
            "model": {
                "type": "minecraft:condition",
                "property": "minecraft:view_entity",
                "on_true": { "type": "minecraft:model", "model": "minecraft:item/view_entity_condition_view" },
                "on_false": { "type": "minecraft:model", "model": "minecraft:item/view_entity_condition" }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "view_entity_condition",
        &[40, 80, 120, 255],
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "view_entity_condition_view",
        &[120, 80, 40, 255],
    );
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(
        root,
        &["view_entity_condition"],
    );
}

pub(super) fn write_runtime_extended_view_condition_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("extended_view_condition.json"),
        r#"{
            "model": {
                "type": "minecraft:condition",
                "property": "minecraft:extended_view",
                "on_true": { "type": "minecraft:model", "model": "minecraft:item/extended_view_condition_view" },
                "on_false": { "type": "minecraft:model", "model": "minecraft:item/extended_view_condition" }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "extended_view_condition",
        &[40, 80, 120, 255],
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "extended_view_condition_view",
        &[120, 80, 40, 255],
    );
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(
        root,
        &["extended_view_condition"],
    );
}

pub(super) fn write_runtime_keybind_down_condition_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("keybind_down_condition.json"),
        r#"{
            "model": {
                "type": "minecraft:condition",
                "property": "minecraft:keybind_down",
                "keybind": "key.quickActions",
                "on_true": { "type": "minecraft:model", "model": "minecraft:item/keybind_down_condition_use" },
                "on_false": { "type": "minecraft:model", "model": "minecraft:item/keybind_down_condition" }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "keybind_down_condition",
        &[40, 80, 120, 255],
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "keybind_down_condition_use",
        &[120, 80, 40, 255],
    );
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(
        root,
        &["keybind_down_condition"],
    );
}

pub(super) fn write_runtime_fishing_rod_cast_condition_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("fishing_rod.json"),
        r#"{
            "model": {
                "type": "minecraft:condition",
                "property": "minecraft:fishing_rod/cast",
                "on_true": { "type": "minecraft:model", "model": "minecraft:item/fishing_rod_cast" },
                "on_false": { "type": "minecraft:model", "model": "minecraft:item/fishing_rod" }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(&assets, "fishing_rod", &[40, 80, 120, 255]);
    write_flat_runtime_item_model_and_texture(&assets, "fishing_rod_cast", &[120, 80, 40, 255]);
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(root, &["fishing_rod"]);
}

pub(super) fn write_runtime_cooldown_range_dispatch_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("cooldown_selector.json"),
        r#"{
            "model": {
                "type": "minecraft:range_dispatch",
                "property": "minecraft:cooldown",
                "entries": [
                    {
                        "threshold": 0.725,
                        "model": { "type": "minecraft:model", "model": "minecraft:item/cooldown_selector_active" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/cooldown_selector" }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(&assets, "cooldown_selector", &[40, 80, 120, 255]);
    write_flat_runtime_item_model_and_texture(
        &assets,
        "cooldown_selector_active",
        &[120, 80, 40, 255],
    );
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(root, &["cooldown_selector"]);
}

pub(super) fn write_runtime_time_range_dispatch_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("time_selector.json"),
        r#"{
            "model": {
                "type": "minecraft:range_dispatch",
                "property": "minecraft:time",
                "source": "daytime",
                "wobble": false,
                "scale": 4.0,
                "entries": [
                    {
                        "threshold": 1.0,
                        "model": { "type": "minecraft:model", "model": "minecraft:item/time_selector_evening" }
                    },
                    {
                        "threshold": 2.0,
                        "model": { "type": "minecraft:model", "model": "minecraft:item/time_selector_night" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/time_selector_day" }
            }
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("time_wobbled_selector.json"),
        r#"{
            "model": {
                "type": "minecraft:range_dispatch",
                "property": "minecraft:time",
                "source": "daytime",
                "scale": 4.0,
                "entries": [
                    {
                        "threshold": 3.0,
                        "model": { "type": "minecraft:model", "model": "minecraft:item/time_wobbled_stateful" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/time_wobbled_fallback" }
            }
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("time_random_selector.json"),
        r#"{
            "model": {
                "type": "minecraft:range_dispatch",
                "property": "minecraft:time",
                "source": "random",
                "wobble": false,
                "scale": 4.0,
                "entries": [
                    {
                        "threshold": 0.5,
                        "model": { "type": "minecraft:model", "model": "minecraft:item/time_random_stateful" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/time_random_fallback" }
            }
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("time_moon_phase_selector.json"),
        r#"{
            "model": {
                "type": "minecraft:range_dispatch",
                "property": "minecraft:time",
                "source": "moon_phase",
                "wobble": false,
                "scale": 8.0,
                "entries": [
                    {
                        "threshold": 4.0,
                        "model": { "type": "minecraft:model", "model": "minecraft:item/time_moon_phase_new" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/time_moon_phase_full" }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(&assets, "time_selector_day", &[40, 80, 120, 255]);
    write_flat_runtime_item_model_and_texture(
        &assets,
        "time_selector_evening",
        &[120, 80, 40, 255],
    );
    write_flat_runtime_item_model_and_texture(&assets, "time_selector_night", &[80, 40, 120, 255]);
    write_flat_runtime_item_model_and_texture(
        &assets,
        "time_wobbled_fallback",
        &[45, 75, 115, 255],
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "time_wobbled_stateful",
        &[160, 40, 100, 255],
    );
    write_flat_runtime_item_model_and_texture(&assets, "time_random_fallback", &[50, 70, 110, 255]);
    write_flat_runtime_item_model_and_texture(&assets, "time_random_stateful", &[180, 50, 90, 255]);
    write_flat_runtime_item_model_and_texture(&assets, "time_moon_phase_full", &[40, 70, 130, 255]);
    write_flat_runtime_item_model_and_texture(&assets, "time_moon_phase_new", &[130, 70, 40, 255]);
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(
        root,
        &[
            "time_selector",
            "time_wobbled_selector",
            "time_random_selector",
            "time_moon_phase_selector",
        ],
    );
}

pub(super) fn write_runtime_spawn_compass_range_dispatch_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("spawn_compass.json"),
        r#"{
            "model": {
                "type": "minecraft:range_dispatch",
                "property": "minecraft:compass",
                "target": "spawn",
                "wobble": false,
                "scale": 4.0,
                "entries": [
                    {
                        "threshold": 3.0,
                        "model": { "type": "minecraft:model", "model": "minecraft:item/spawn_compass_east" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/spawn_compass_fallback" }
            }
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("spawn_compass_wobbled.json"),
        r#"{
            "model": {
                "type": "minecraft:range_dispatch",
                "property": "minecraft:compass",
                "target": "spawn",
                "scale": 4.0,
                "entries": [
                    {
                        "threshold": 3.5,
                        "model": { "type": "minecraft:model", "model": "minecraft:item/spawn_compass_wobbled_east" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/spawn_compass_wobbled_fallback" }
            }
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("spawn_compass_invalid_spin.json"),
        r#"{
            "model": {
                "type": "minecraft:range_dispatch",
                "property": "minecraft:compass",
                "target": "spawn",
                "scale": 1.0,
                "entries": [
                    {
                        "threshold": 0.1,
                        "model": { "type": "minecraft:model", "model": "minecraft:item/spawn_compass_invalid_spin" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/spawn_compass_invalid_fallback" }
            }
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("spawn_compass_none_spin.json"),
        r#"{
            "model": {
                "type": "minecraft:range_dispatch",
                "property": "minecraft:compass",
                "target": "none",
                "scale": 1.0,
                "entries": [
                    {
                        "threshold": 0.1,
                        "model": { "type": "minecraft:model", "model": "minecraft:item/spawn_compass_none_spin" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/spawn_compass_none_fallback" }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "spawn_compass_fallback",
        &[40, 80, 120, 255],
    );
    write_flat_runtime_item_model_and_texture(&assets, "spawn_compass_east", &[120, 80, 40, 255]);
    write_flat_runtime_item_model_and_texture(
        &assets,
        "spawn_compass_wobbled_fallback",
        &[45, 75, 115, 255],
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "spawn_compass_wobbled_east",
        &[140, 90, 50, 255],
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "spawn_compass_invalid_fallback",
        &[35, 55, 95, 255],
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "spawn_compass_invalid_spin",
        &[190, 70, 80, 255],
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "spawn_compass_none_fallback",
        &[30, 45, 90, 255],
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "spawn_compass_none_spin",
        &[185, 85, 105, 255],
    );
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(
        root,
        &[
            "spawn_compass",
            "spawn_compass_wobbled",
            "spawn_compass_invalid_spin",
            "spawn_compass_none_spin",
        ],
    );
}

pub(super) fn write_runtime_lodestone_compass_range_dispatch_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("lodestone_compass.json"),
        r#"{
            "model": {
                "type": "minecraft:range_dispatch",
                "property": "minecraft:compass",
                "target": "lodestone",
                "wobble": false,
                "scale": 4.0,
                "entries": [
                    {
                        "threshold": 3.0,
                        "model": { "type": "minecraft:model", "model": "minecraft:item/lodestone_compass_east" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/lodestone_compass_fallback" }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "lodestone_compass_fallback",
        &[40, 120, 80, 255],
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "lodestone_compass_east",
        &[120, 40, 80, 255],
    );
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(root, &["lodestone_compass"]);
}

pub(super) fn write_runtime_recovery_compass_range_dispatch_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("recovery_compass.json"),
        r#"{
            "model": {
                "type": "minecraft:range_dispatch",
                "property": "minecraft:compass",
                "target": "recovery",
                "wobble": false,
                "scale": 4.0,
                "entries": [
                    {
                        "threshold": 3.0,
                        "model": { "type": "minecraft:model", "model": "minecraft:item/recovery_compass_east" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/recovery_compass_fallback" }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "recovery_compass_fallback",
        &[40, 80, 120, 255],
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "recovery_compass_east",
        &[120, 80, 40, 255],
    );
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(root, &["recovery_compass"]);
}

pub(super) fn write_runtime_selected_condition_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("selected_condition.json"),
        r#"{
            "model": {
                "type": "minecraft:condition",
                "property": "minecraft:selected",
                "on_true": { "type": "minecraft:model", "model": "minecraft:item/selected_condition_selected" },
                "on_false": { "type": "minecraft:model", "model": "minecraft:item/selected_condition" }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(&assets, "selected_condition", &[40, 80, 120, 255]);
    write_flat_runtime_item_model_and_texture(
        &assets,
        "selected_condition_selected",
        &[120, 80, 40, 255],
    );
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(root, &["selected_condition"]);
}

pub(super) fn write_runtime_carried_condition_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("carried_condition.json"),
        r#"{
            "model": {
                "type": "minecraft:condition",
                "property": "minecraft:carried",
                "on_true": { "type": "minecraft:model", "model": "minecraft:item/carried_condition_carried" },
                "on_false": { "type": "minecraft:model", "model": "minecraft:item/carried_condition" }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(&assets, "carried_condition", &[40, 120, 80, 255]);
    write_flat_runtime_item_model_and_texture(
        &assets,
        "carried_condition_carried",
        &[120, 40, 80, 255],
    );
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(root, &["carried_condition"]);
}

pub(super) fn write_runtime_special_foil_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    for item_id in ["clock", "compass", "spyglass"] {
        write_runtime_json(
            &assets.join("items").join(format!("{item_id}.json")),
            &format!(
                r#"{{
                    "model": {{ "type": "minecraft:model", "model": "minecraft:item/{item_id}" }}
                }}"#
            ),
        );
        write_flat_runtime_item_model_and_texture(&assets, item_id, &[40, 80, 120, 255]);
    }
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    write_runtime_json(
        &root
            .join("sources")
            .join(bbb_client_assets::MC_VERSION)
            .join("data")
            .join("minecraft")
            .join("tags")
            .join("item")
            .join("compasses.json"),
        r#"{
            "replace": true,
            "values": ["minecraft:compass"]
        }"#,
    );
    crate::item_registry_test_fixture::write_item_registry_fixture(
        root,
        &["clock", "compass", "spyglass"],
    );
}

pub(super) fn write_runtime_special_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{"sources": []}"#,
    );
    write_runtime_json(
        &assets.join("items").join("special_trident.json"),
        r#"{
            "model": {
                "type": "minecraft:special",
                "base": "minecraft:item/special_trident_base",
                "model": { "type": "minecraft:trident" }
            }
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("bundle.json"),
        r#"{
            "model": {
                "type": "minecraft:condition",
                "property": "minecraft:bundle/has_selected_item",
                "on_true": {
                    "type": "minecraft:composite",
                    "models": [
                        { "type": "minecraft:model", "model": "minecraft:item/bundle_open_back" },
                        { "type": "minecraft:bundle/selected_item" },
                        { "type": "minecraft:model", "model": "minecraft:item/bundle_open_front" }
                    ]
                },
                "on_false": { "type": "minecraft:model", "model": "minecraft:item/bundle" }
            }
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("carried_special.json"),
        r#"{
            "model": {
                "type": "minecraft:condition",
                "property": "minecraft:carried",
                "on_true": {
                    "type": "minecraft:special",
                    "base": "minecraft:item/carried_special_base",
                    "model": { "type": "minecraft:bell" }
                },
                "on_false": {
                    "type": "minecraft:model",
                    "model": "minecraft:item/carried_special"
                }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(&assets, "special_trident_base", &[40, 80, 120, 255]);
    write_flat_runtime_item_model_and_texture(&assets, "bundle", &[40, 80, 120, 255]);
    write_flat_runtime_item_model_and_texture(&assets, "bundle_open_back", &[50, 90, 130, 255]);
    write_flat_runtime_item_model_and_texture(&assets, "bundle_open_front", &[60, 100, 140, 255]);
    write_flat_runtime_item_model_and_texture(&assets, "carried_special", &[70, 110, 150, 255]);
    write_flat_runtime_item_model_and_texture(
        &assets,
        "carried_special_base",
        &[80, 120, 160, 255],
    );
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(
        root,
        &["bundle", "special_trident", "carried_special"],
    );
}

pub(super) fn write_runtime_trim_material_select_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("trim_selector.json"),
        r#"{
            "model": {
                "type": "minecraft:select",
                "property": "minecraft:trim_material",
                "cases": [
                    {
                        "when": "minecraft:quartz",
                        "model": { "type": "minecraft:model", "model": "minecraft:item/trim_selector_quartz" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/trim_selector" }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(&assets, "trim_selector", &[40, 80, 120, 255]);
    write_flat_runtime_item_model_and_texture(&assets, "trim_selector_quartz", &[120, 80, 40, 255]);
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(root, &["trim_selector"]);
}

pub(super) fn write_flat_runtime_item_model_and_texture(
    assets: &Path,
    model_id: &str,
    rgba: &[u8],
) {
    write_runtime_json(
        &assets
            .join("models")
            .join("item")
            .join(format!("{model_id}.json")),
        &format!(
            r#"{{
            "parent": "minecraft:builtin/generated",
            "textures": {{
                "layer0": "minecraft:item/{model_id}"
            }}
        }}"#
        ),
    );
    write_runtime_png(
        &assets
            .join("textures")
            .join("item")
            .join(format!("{model_id}.png")),
        rgba,
    );
}

pub(super) fn write_runtime_game_mode_switcher_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    for (item_id, rgba) in [
        ("grass_block", [80, 160, 80, 255]),
        ("iron_sword", [180, 180, 190, 255]),
        ("map", [210, 190, 120, 255]),
        ("ender_eye", [80, 170, 140, 255]),
    ] {
        write_runtime_json(
            &assets.join("items").join(format!("{item_id}.json")),
            &format!(
                r#"{{
                    "model": {{
                        "type": "minecraft:model",
                        "model": "minecraft:item/{item_id}"
                    }}
                }}"#
            ),
        );
        write_flat_runtime_item_model_and_texture(&assets, item_id, &rgba);
    }
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(
        root,
        &["grass_block", "iron_sword", "map", "ender_eye"],
    );
}

pub(super) fn runtime_assets_dir(root: &Path) -> PathBuf {
    root.join("sources")
        .join(bbb_client_assets::MC_VERSION)
        .join("assets")
        .join("minecraft")
}

pub(super) fn write_runtime_json(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

pub(super) fn write_runtime_png(path: &Path, rgba: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    image::save_buffer(path, rgba, 1, 1, image::ColorType::Rgba8).unwrap();
}

pub(super) fn unique_runtime_temp_dir(label: &str) -> PathBuf {
    static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let process = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("bbb-client-runtime-{label}-{process}-{nanos}-{id}"))
}

/// Owned defaults for the F3 overlay inputs a test does not set.
#[derive(Default)]
pub(super) struct HudDebugTestDefaults {
    terrain_textures: TerrainTextureState,
    renderer_counters: bbb_client_render::RendererCounters,
    fps_sampler: HudDebugFpsSampler,
    network_sampler: HudDebugNetworkSampler,
    tps_sampler: HudDebugTpsSampler,
    net_counters: NetCounters,
    audio_counters: AudioCounters,
}

impl HudDebugTestDefaults {
    pub(super) fn inputs<'a>(
        &'a self,
        input: &'a ClientInputState,
        world: &'a WorldStore,
        camera_pose: Option<CameraPose>,
    ) -> HudDebugOverlayInputs<'a> {
        let surface_size = winit::dpi::PhysicalSize::new(320, 240);
        HudDebugOverlayInputs {
            input,
            world,
            visible_entity_render_state_count: world.entity_count(),
            camera_pose,
            interaction_camera_pose: camera_pose,
            targeted_entity_id: match crosshair_target_from_camera(world, camera_pose) {
                Some(CrosshairTarget::Entity(hit)) => Some(hit.entity_id),
                _ => None,
            },
            entity_partial_tick: 1.0,
            item_runtime: None,
            terrain_textures: &self.terrain_textures,
            renderer_counters: &self.renderer_counters,
            gpu_utilization: None,
            improved_transparency: false,
            cloud_shape: None,
            texture_filtering: bbb_client_render::TerrainTextureFiltering::None,
            max_anisotropy: 1,
            render_distance_chunks: VANILLA_MAX_RENDER_DISTANCE_CHUNKS,
            surface_size,
            physical_surface_size: surface_size,
            fps_sampler: &self.fps_sampler,
            client_framerate_limit: VANILLA_UNLIMITED_FRAMERATE_LIMIT,
            client_vsync: true,
            network_sampler: &self.network_sampler,
            tps_sampler: &self.tps_sampler,
            net_counters: &self.net_counters,
            audio_counters: &self.audio_counters,
            current_post_effect_id: input.camera_post_effect_id(),
            profiler_chart: None,
            hud_info_enabled: false,
        }
    }
}
