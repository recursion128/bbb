use super::*;

#[test]
fn fishing_hook_render_source_projects_owner_hand_inputs() {
    let mut store = WorldStore::new();
    store.apply_add_entity(ProtocolAddEntity {
        id: 10,
        entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
        position: ProtocolVec3d {
            x: 2.0,
            y: 64.0,
            z: 3.0,
        },
        y_rot: 90.0,
        ..protocol_add_entity_with_type_data(10, VANILLA_ENTITY_TYPE_PLAYER_ID, 0)
    });
    store.apply_add_entity(ProtocolAddEntity {
        id: 20,
        entity_type_id: VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
        position: ProtocolVec3d {
            x: 6.0,
            y: 62.0,
            z: -1.0,
        },
        data: 10,
        ..protocol_add_entity_with_type_data(20, VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID, 10)
    });

    let source = store
        .fishing_hook_render_source_at_partial_tick(20, 1.0)
        .expect("player-owned fishing hook source");
    assert_entity_vec3_close(
        source.hook_position,
        EntityVec3 {
            x: 6.0,
            y: 62.0,
            z: -1.0,
        },
    );
    assert_eq!(source.owner_id, 10);
    assert_entity_vec3_close(
        source.owner_eye_position,
        EntityVec3 {
            x: 2.0,
            y: 65.62,
            z: 3.0,
        },
    );
    assert_eq!(source.owner_body_y_rot, 90.0);
    assert_eq!(source.owner_scale, 1.0);
    assert!(!source.owner_is_crouching);
    // No rod in the main hand: vanilla chooses the opposite of the main (right) arm.
    assert!(source.holding_arm_left);
    assert_eq!(source.owner_attack_anim, 0.0);

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 10,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::MainHand,
            item: ItemStackSummary {
                item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_FISHING_ROD_ID),
                count: 1,
                component_patch: Default::default(),
            },
        }],
    }));
    let accessors = vanilla_entity_metadata_entity_accessors().unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 10,
        values: vec![protocol_pose_data(
            accessors.pose_data_id,
            bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROUCHING_ID,
        )],
    }));
    let source = store
        .fishing_hook_render_source_at_partial_tick(20, 1.0)
        .unwrap();
    assert!(!source.holding_arm_left);
    assert!(source.owner_is_crouching);

    assert!(store
        .fishing_hook_render_source_at_partial_tick(10, 1.0)
        .is_none());
}

fn entity_metadata_accessors() -> VanillaEntityMetadataAccessors {
    vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors must resolve")
}

fn entity_shared_flags_data(flags: i8) -> ProtocolEntityDataValue {
    let accessors = entity_metadata_accessors();
    assert_eq!(accessors.shared_flags_default, 0);
    protocol_byte_data(accessors.shared_flags_data_id, flags)
}

fn baby_data(entity_type_id: i32, is_baby: bool) -> ProtocolEntityDataValue {
    let (data_id, default) = vanilla_entity_metadata_baby_accessor_for_entity_type(entity_type_id)
        .expect("generated nearest baby accessor must resolve");
    assert!(!default);
    protocol_bool_data(data_id, is_baby)
}

#[test]
fn entity_model_sources_project_boat_bubble_angle_from_bubble_time() {
    let accessors = vanilla_entity_metadata_abstract_boat_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    )
    .expect("generated AbstractBoat metadata accessors must resolve");

    let assert_close = |actual: f32, expected: f32, label: &str| {
        assert!(
            (actual - expected).abs() < 1.0e-6,
            "{label}: expected {expected}, got {actual}"
        );
    };
    let bubble = |store: &WorldStore, partial_tick: f32| -> f32 {
        store
            .entity_model_sources_at_partial_tick(partial_tick)
            .into_iter()
            .find(|source| source.entity_id == 22)
            .expect("boat source")
            .boat_bubble_angle
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        22,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert_eq!(bubble(&store, 1.0), 0.0);

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 22,
        values: vec![protocol_int_data(accessors.bubble_time_data_id, 60)],
    }));
    store.advance_entity_client_animations(1);
    let first_angle = 10.0 * (0.5_f32).sin() * 0.05;
    assert_close(bubble(&store, 0.0), 0.0, "first tick previous angle");
    assert_close(bubble(&store, 1.0), first_angle, "first tick current angle");

    store.advance_entity_client_animations(1);
    let second_angle = 10.0 * (1.0_f32).sin() * 0.1;
    assert_close(
        bubble(&store, 0.5),
        first_angle + (second_angle - first_angle) * 0.5,
        "second tick partial angle",
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 22,
        values: vec![protocol_int_data(
            accessors.bubble_time_data_id,
            accessors.bubble_time_default,
        )],
    }));
    store.advance_entity_client_animations(1);
    assert_close(bubble(&store, 1.0), 0.0, "cleared bubble angle");
}

#[test]
fn entity_model_sources_project_invisible_to_player_for_spectator_viewer() {
    let invisible_mask = entity_metadata_accessors().invisible_mask;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        35,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 35,
        values: vec![entity_shared_flags_data(invisible_mask)],
    }));
    store.apply_add_entity(protocol_add_entity_with_type(
        36,
        VANILLA_ENTITY_TYPE_MINECART_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 36,
        values: vec![entity_shared_flags_data(invisible_mask)],
    }));

    let survival = store.entity_model_sources_at_partial_tick(1.0);
    let survival_chicken = survival
        .iter()
        .find(|source| source.entity_id == 35)
        .unwrap();
    let survival_minecart = survival
        .iter()
        .find(|source| source.entity_id == 36)
        .unwrap();
    assert!(survival_chicken.invisible_to_player);
    assert!(survival_minecart.invisible_to_player);

    store.apply_game_event(ProtocolGameEvent {
        event_id: 3,
        param: 3.0,
    });

    let spectator = store.entity_model_sources_at_partial_tick(1.0);
    let spectator_chicken = spectator
        .iter()
        .find(|source| source.entity_id == 35)
        .unwrap();
    let spectator_minecart = spectator
        .iter()
        .find(|source| source.entity_id == 36)
        .unwrap();
    assert!(!spectator_chicken.invisible_to_player);
    assert!(spectator_minecart.invisible_to_player);
}

#[test]
fn entity_model_sources_project_same_team_friendly_invisible_visibility() {
    let invisible_mask = entity_metadata_accessors().invisible_mask;

    let mut store = WorldStore::new();
    let local_uuid = Uuid::from_u128(0x33345678123456781234567812345678);
    let chicken_uuid = Uuid::from_u128(0x44345678123456781234567812345678);
    let minecart_uuid = Uuid::from_u128(0x55345678123456781234567812345678);

    store.apply_login(&protocol_play_login(40));
    let mut local_player = protocol_add_entity_with_type(40, VANILLA_ENTITY_TYPE_PLAYER_ID);
    local_player.uuid = local_uuid;
    store.apply_add_entity(local_player);
    store.apply_player_info_update(ProtocolPlayerInfoUpdate {
        actions: vec![ProtocolPlayerInfoAction::AddPlayer],
        entries: vec![protocol_player_info_entry_with_mode(
            local_uuid,
            ProtocolGameType::Survival,
        )],
    });

    let mut chicken = protocol_add_entity_with_type(41, VANILLA_ENTITY_TYPE_CHICKEN_ID);
    chicken.uuid = chicken_uuid;
    store.apply_add_entity(chicken);
    let mut minecart = protocol_add_entity_with_type(42, VANILLA_ENTITY_TYPE_MINECART_ID);
    minecart.uuid = minecart_uuid;
    store.apply_add_entity(minecart);
    for id in [41, 42] {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![entity_shared_flags_data(invisible_mask)],
        }));
    }

    assert!(store.apply_set_player_team(ProtocolSetPlayerTeam {
        name: "green".to_string(),
        method: ProtocolPlayerTeamMethod::Add.into(),
        parameters: Some(ProtocolPlayerTeamParameters {
            display_name: bbb_protocol::RichTextComponent::plain("Green"),
            options: 0,
            nametag_visibility: ProtocolTeamVisibility::Always,
            collision_rule: ProtocolTeamCollisionRule::Always,
            color: ProtocolChatFormatting::Green,
            player_prefix: bbb_protocol::RichTextComponent::plain(""),
            player_suffix: bbb_protocol::RichTextComponent::plain(""),
        }),
        players: vec![
            "PickTarget".to_string(),
            chicken_uuid.to_string(),
            minecart_uuid.to_string(),
        ],
    }));
    assert_eq!(store.local_player_team_color_rgb(), Some(0x55ff55));

    let hidden = store.entity_model_sources_at_partial_tick(1.0);
    assert!(
        hidden
            .iter()
            .find(|source| source.entity_id == 41)
            .unwrap()
            .invisible_to_player
    );

    assert!(store.apply_set_player_team(ProtocolSetPlayerTeam {
        name: "green".to_string(),
        method: ProtocolPlayerTeamMethod::Change.into(),
        parameters: Some(ProtocolPlayerTeamParameters {
            display_name: bbb_protocol::RichTextComponent::plain("Green"),
            options: 2,
            nametag_visibility: ProtocolTeamVisibility::Always,
            collision_rule: ProtocolTeamCollisionRule::Always,
            color: ProtocolChatFormatting::Green,
            player_prefix: bbb_protocol::RichTextComponent::plain(""),
            player_suffix: bbb_protocol::RichTextComponent::plain(""),
        }),
        players: Vec::new(),
    }));

    let friendly_visible = store.entity_model_sources_at_partial_tick(1.0);
    let friendly_chicken = friendly_visible
        .iter()
        .find(|source| source.entity_id == 41)
        .unwrap();
    let friendly_minecart = friendly_visible
        .iter()
        .find(|source| source.entity_id == 42)
        .unwrap();
    assert!(!friendly_chicken.invisible_to_player);
    assert!(friendly_minecart.invisible_to_player);
}

#[test]
fn entity_model_sources_project_glowing_shared_flag() {
    let glowing_mask = entity_metadata_accessors().glowing_mask;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        37,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 37,
        values: vec![entity_shared_flags_data(glowing_mask)],
    }));

    let sources = store.entity_model_sources_at_partial_tick(1.0);
    let chicken = sources
        .iter()
        .find(|source| source.entity_id == 37)
        .unwrap();
    assert!(chicken.appears_glowing);
    assert_eq!(chicken.outline_color, 0xffff_ffff);
    assert!(!chicken.invisible_to_player);
}

#[test]
fn entity_model_sources_project_team_outline_color() {
    let glowing_mask = entity_metadata_accessors().glowing_mask;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        37,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    let chicken_uuid = default_entity_uuid();
    let player_uuid = Uuid::from_u128(0x22345678123456781234567812345678);
    let mut player = protocol_add_entity_with_type(38, VANILLA_ENTITY_TYPE_PLAYER_ID);
    player.uuid = player_uuid;
    store.apply_add_entity(player);
    store.apply_player_info_update(ProtocolPlayerInfoUpdate {
        actions: vec![ProtocolPlayerInfoAction::AddPlayer],
        entries: vec![protocol_player_info_entry_with_mode(
            player_uuid,
            ProtocolGameType::Survival,
        )],
    });
    assert!(store.apply_set_player_team(ProtocolSetPlayerTeam {
        name: "green".to_string(),
        method: ProtocolPlayerTeamMethod::Add.into(),
        parameters: Some(ProtocolPlayerTeamParameters {
            display_name: bbb_protocol::RichTextComponent::plain("Green"),
            options: 0,
            nametag_visibility: ProtocolTeamVisibility::Always,
            collision_rule: ProtocolTeamCollisionRule::Always,
            color: ProtocolChatFormatting::Green,
            player_prefix: bbb_protocol::RichTextComponent::plain(""),
            player_suffix: bbb_protocol::RichTextComponent::plain(""),
        }),
        players: vec![chicken_uuid.to_string(), "PickTarget".to_string()],
    }));
    for id in [37, 38] {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![entity_shared_flags_data(glowing_mask)],
        }));
    }

    let sources = store.entity_model_sources_at_partial_tick(1.0);
    let chicken = sources
        .iter()
        .find(|source| source.entity_id == 37)
        .unwrap();
    let player = sources
        .iter()
        .find(|source| source.entity_id == 38)
        .unwrap();

    assert!(chicken.appears_glowing);
    assert_eq!(chicken.outline_color, 0xff55_ff55);
    assert!(player.appears_glowing);
    assert_eq!(player.outline_color, 0xff55_ff55);
    assert_eq!(store.entity_team_color_rgb(37), Some(0x55ff55));
    assert_eq!(store.entity_team_color_rgb(38), Some(0x55ff55));
    assert_eq!(store.entity_team_color_rgb(999), None);
}

#[test]
fn entity_model_sources_project_worn_armor_materials() {
    use std::collections::BTreeMap;
    // The registry-derived item id → armor material table (installed by the native layer).
    let iron_helmet = 800;
    let iron_chestplate = 801;
    let diamond_leggings = 802;
    let gold_boots = 803;
    let stone_sword = 900;

    let mut store = WorldStore::new();
    store.set_item_armor_materials(BTreeMap::from([
        (iron_helmet, ArmorMaterialKind::Iron),
        (iron_chestplate, ArmorMaterialKind::Iron),
        (diamond_leggings, ArmorMaterialKind::Diamond),
        (gold_boots, ArmorMaterialKind::Gold),
    ]));
    store.apply_add_entity(protocol_add_entity_with_type(
        50,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    ));

    // A bare zombie projects no worn armor.
    let bare = store.entity_model_sources_at_partial_tick(1.0);
    assert_eq!(bare[0].head_armor, None);
    assert_eq!(bare[0].chest_armor, None);
    assert!(!bare[0].head_armor_foil);
    assert!(!bare[0].chest_armor_foil);

    fn armor_item(item_id: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count: 1,
            component_patch: Default::default(),
        }
    }
    fn enchanted_armor_item(
        item_id: i32,
        enchantment_glint_override: Option<bool>,
    ) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count: 1,
            component_patch: DataComponentPatchSummary {
                enchantments: vec![ItemEnchantmentSummary {
                    holder_id: 12,
                    level: 1,
                }],
                enchantment_glint_override,
                ..Default::default()
            },
        }
    }

    // Equip all four armor slots; a held sword fills MainHand but is not armor.
    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 50,
        slots: vec![
            EquipmentSlotUpdate {
                slot: EquipmentSlot::Head,
                item: armor_item(iron_helmet),
            },
            EquipmentSlotUpdate {
                slot: EquipmentSlot::Chest,
                item: enchanted_armor_item(iron_chestplate, None),
            },
            EquipmentSlotUpdate {
                slot: EquipmentSlot::Legs,
                item: enchanted_armor_item(diamond_leggings, Some(false)),
            },
            EquipmentSlotUpdate {
                slot: EquipmentSlot::Feet,
                item: armor_item(gold_boots),
            },
            EquipmentSlotUpdate {
                slot: EquipmentSlot::MainHand,
                item: armor_item(stone_sword),
            },
        ],
    }));

    let sources = store.entity_model_sources_at_partial_tick(1.0);
    assert_eq!(sources[0].head_armor, Some(ArmorMaterialKind::Iron));
    assert_eq!(sources[0].chest_armor, Some(ArmorMaterialKind::Iron));
    assert_eq!(sources[0].legs_armor, Some(ArmorMaterialKind::Diamond));
    assert_eq!(sources[0].feet_armor, Some(ArmorMaterialKind::Gold));
    assert!(!sources[0].head_armor_foil);
    assert!(sources[0].chest_armor_foil);
    assert!(
        !sources[0].legs_armor_foil,
        "enchantment_glint_override=false wins over non-empty enchantments"
    );
    assert!(!sources[0].feet_armor_foil);

    // A non-armor item (the held sword, absent from the armor map) leaves its slot bare; clearing the
    // helmet (empty stack) drops the head armor.
    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 50,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Head,
            item: ItemStackSummary::empty(),
        }],
    }));
    let sources = store.entity_model_sources_at_partial_tick(1.0);
    assert_eq!(sources[0].head_armor, None);
    assert_eq!(sources[0].chest_armor, Some(ArmorMaterialKind::Iron));
    assert!(!sources[0].head_armor_foil);
    assert!(sources[0].chest_armor_foil);
}

#[test]
fn entity_model_sources_apply_equippable_asset_and_slot_patches_to_worn_armor() {
    use std::collections::BTreeMap;

    const DEFAULT_IRON_HELMET: i32 = 804;
    const PATCHED_ITEM: i32 = 805;
    let mut store = WorldStore::new();
    store.set_item_armor_materials(BTreeMap::from([(
        DEFAULT_IRON_HELMET,
        ArmorMaterialKind::Iron,
    )]));
    store.apply_add_entity(protocol_add_entity_with_type(
        51,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    ));

    let equip_head = |item: ItemStackSummary| ProtocolSetEquipment {
        entity_id: 51,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Head,
            item,
        }],
    };
    let equippable = |slot_id: i32, asset_id: Option<&str>| EquippableSummary {
        slot_id,
        equip_sound: SoundEventSummary {
            registry_id: Some(0),
            sound_id: None,
            fixed_range_bits: None,
        },
        asset_id: asset_id.map(str::to_string),
        camera_overlay: None,
        allowed_entities: None,
        dispensable: true,
        swappable: true,
        damage_on_hurt: true,
        equip_on_interact: false,
        can_be_sheared: false,
        shearing_sound: SoundEventSummary {
            registry_id: Some(1),
            sound_id: None,
            fixed_range_bits: None,
        },
    };
    let source_material = |store: &WorldStore| {
        store
            .entity_model_source_at_partial_tick(51, 1.0)
            .expect("zombie source")
            .head_armor
    };

    assert!(store.apply_set_equipment(equip_head(ItemStackSummary {
        item_id: Some(PATCHED_ITEM),
        count: 1,
        component_patch: DataComponentPatchSummary {
            added_type_ids: vec![
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
            ],
            equippable: Some(equippable(4, Some("minecraft:diamond"))),
            ..Default::default()
        },
    })));
    assert_eq!(source_material(&store), Some(ArmorMaterialKind::Diamond));

    assert!(store.apply_set_equipment(equip_head(ItemStackSummary {
        item_id: Some(DEFAULT_IRON_HELMET),
        count: 1,
        component_patch: DataComponentPatchSummary {
            added_type_ids: vec![
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
            ],
            equippable: Some(equippable(3, Some("minecraft:diamond"))),
            ..Default::default()
        },
    })));
    assert_eq!(
        source_material(&store),
        None,
        "an equippable component for the chest slot suppresses the item's default head asset"
    );

    assert!(store.apply_set_equipment(equip_head(ItemStackSummary {
        item_id: Some(DEFAULT_IRON_HELMET),
        count: 1,
        component_patch: DataComponentPatchSummary {
            removed_type_ids: vec![
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
            ],
            ..Default::default()
        },
    })));
    assert_eq!(
        source_material(&store),
        None,
        "removing equippable suppresses the registry-derived default asset"
    );
}

#[test]
fn entity_model_sources_project_effective_equipment_asset_ids_and_exact_trim_names() {
    use std::collections::BTreeMap;

    const DEFAULT_HEAD_ITEM: i32 = 820;
    const PATCHED_ITEM: i32 = 821;
    const SADDLE_ITEM: i32 = 822;
    const BODY_ITEM: i32 = 823;
    const SADDLE_DYE: i32 = 0x12_34_56;
    const BODY_DYE: i32 = 0x65_43_21;

    fn equippable(slot_id: i32, asset_id: Option<&str>) -> EquippableSummary {
        EquippableSummary {
            slot_id,
            equip_sound: SoundEventSummary {
                registry_id: Some(0),
                sound_id: None,
                fixed_range_bits: None,
            },
            asset_id: asset_id.map(str::to_string),
            camera_overlay: None,
            allowed_entities: None,
            dispensable: true,
            swappable: true,
            damage_on_hurt: true,
            equip_on_interact: false,
            can_be_sheared: false,
            shearing_sound: SoundEventSummary {
                registry_id: Some(1),
                sound_id: None,
                fixed_range_bits: None,
            },
        }
    }

    let mut store = WorldStore::new();
    store.set_default_item_equipment_assets(
        7,
        vec![
            "minecraft:diamond".to_string(),
            "example:custom".to_string(),
            "minecraft:saddle".to_string(),
        ],
        BTreeMap::from([
            (DEFAULT_HEAD_ITEM, EquipmentAssetId::new(7, 0)),
            (SADDLE_ITEM, EquipmentAssetId::new(7, 2)),
            (BODY_ITEM, EquipmentAssetId::new(7, 1)),
        ]),
    );
    store.set_default_item_equipment_slots(BTreeMap::from([
        (DEFAULT_HEAD_ITEM, crate::ItemEquipmentSlot::Head),
        (SADDLE_ITEM, crate::ItemEquipmentSlot::Saddle),
        (BODY_ITEM, crate::ItemEquipmentSlot::Body),
    ]));
    // The compatibility material is deliberately different: compact identity
    // and trim selection must use the exact equipment asset table.
    store.set_item_armor_materials(BTreeMap::from([(
        DEFAULT_HEAD_ITEM,
        ArmorMaterialKind::Iron,
    )]));
    store.apply_add_entity(protocol_add_entity_with_type(
        54,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        55,
        VANILLA_ENTITY_TYPE_PIG_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        56,
        VANILLA_ENTITY_TYPE_WOLF_ID,
    ));

    let direct_trim = DataComponentPatchSummary {
        armor_trim_material_direct: Some(Box::new(TrimMaterialSummary {
            asset_name: "ruby".to_string(),
            override_armor_assets: BTreeMap::from([(
                "example:custom".to_string(),
                "ruby_custom".to_string(),
            )]),
            description: String::new(),
            description_styled: None,
            description_nbt: None,
        })),
        armor_trim_pattern_direct: Some(Box::new(TrimPatternSummary {
            asset_id: "example:spiral".to_string(),
            description: String::new(),
            description_styled: None,
            description_nbt: None,
            decal: true,
        })),
        ..Default::default()
    };
    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 54,
        slots: vec![
            EquipmentSlotUpdate {
                slot: EquipmentSlot::Head,
                item: ItemStackSummary {
                    item_id: Some(PATCHED_ITEM),
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added_type_ids: vec![
                            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
                        ],
                        equippable: Some(equippable(4, Some("example:custom"))),
                        enchantment_glint_override: Some(true),
                        dyed_color: Some(0x12_34_56),
                        ..direct_trim
                    },
                },
            },
            EquipmentSlotUpdate {
                slot: EquipmentSlot::Chest,
                item: ItemStackSummary {
                    item_id: Some(PATCHED_ITEM),
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added_type_ids: vec![
                            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
                        ],
                        // Wings/SimpleEquipmentLayer do not require this
                        // component slot to match their source slot.
                        equippable: Some(equippable(4, Some("example:custom"))),
                        ..Default::default()
                    },
                },
            },
        ],
    }));
    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 55,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: ItemStackSummary {
                item_id: Some(SADDLE_ITEM),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    dyed_color: Some(SADDLE_DYE),
                    ..Default::default()
                },
            },
        }],
    }));
    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 56,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Body,
            item: ItemStackSummary {
                item_id: Some(BODY_ITEM),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    dyed_color: Some(BODY_DYE),
                    ..Default::default()
                },
            },
        }],
    }));

    let zombie = store
        .entity_model_source_at_partial_tick(54, 1.0)
        .expect("zombie source");
    assert_eq!(
        zombie.head_equipment_asset,
        Some(EquipmentAssetId::new(7, 1))
    );
    assert_eq!(
        zombie.chest_wings_equipment_asset,
        Some(EquipmentAssetId::new(7, 1))
    );
    assert_eq!(zombie.head_armor, None);
    assert_eq!(zombie.head_armor_dye, Some(0x12_34_56));
    assert!(zombie.head_armor_foil);
    assert_eq!(
        zombie.head_armor_trim,
        Some(ArmorTrimSprite {
            sprite_id: "example:trims/entity/humanoid/spiral_ruby_custom".to_string(),
            decal: true,
        })
    );
    let pig = store
        .entity_model_source_at_partial_tick(55, 1.0)
        .expect("pig source");
    assert_eq!(
        pig.pig_saddle_equipment_asset,
        Some(EquipmentAssetId::new(7, 2))
    );
    assert_eq!(pig.pig_saddle_dye, Some(SADDLE_DYE));
    let wolf = store
        .entity_model_source_at_partial_tick(56, 1.0)
        .expect("wolf source");
    assert_eq!(
        wolf.wolf_body_equipment_asset,
        Some(EquipmentAssetId::new(7, 1))
    );
    assert_eq!(wolf.wolf_body_armor_dye, Some(BODY_DYE));

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 54,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Head,
            item: ItemStackSummary {
                item_id: Some(DEFAULT_HEAD_ITEM),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    removed_type_ids: vec![
                        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
                    ],
                    enchantment_glint_override: Some(true),
                    ..Default::default()
                },
            },
        }],
    }));
    let removed = store
        .entity_model_source_at_partial_tick(54, 1.0)
        .expect("zombie source after removal");
    assert_eq!(removed.head_equipment_asset, None);
    assert_eq!(removed.head_armor_trim, None);
    assert!(!removed.head_armor_foil);

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 54,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Head,
            item: ItemStackSummary {
                item_id: Some(DEFAULT_HEAD_ITEM),
                count: 1,
                component_patch: Default::default(),
            },
        }],
    }));
    let defaulted = store
        .entity_model_source_at_partial_tick(54, 1.0)
        .expect("zombie source with prototype default");
    assert_eq!(
        defaulted.head_equipment_asset,
        Some(EquipmentAssetId::new(7, 0))
    );
    assert_eq!(
        defaulted.head_armor,
        Some(ArmorMaterialKind::Iron),
        "legacy material state remains intact until the renderer consumer migrates"
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 54,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Head,
            item: ItemStackSummary {
                item_id: Some(PATCHED_ITEM),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    equippable: Some(equippable(3, Some("example:custom"))),
                    ..Default::default()
                },
            },
        }],
    }));
    assert_eq!(
        store
            .entity_model_source_at_partial_tick(54, 1.0)
            .unwrap()
            .head_equipment_asset,
        None,
        "humanoid armor requires the effective Equippable slot to match"
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 56,
        values: vec![baby_data(VANILLA_ENTITY_TYPE_WOLF_ID, true)],
    }));
    assert_eq!(
        store
            .entity_model_source_at_partial_tick(56, 1.0)
            .unwrap()
            .wolf_body_equipment_asset,
        None,
        "the existing adult-only wolf armor model gate remains authoritative"
    );
    assert_eq!(
        store
            .entity_model_source_at_partial_tick(56, 1.0)
            .unwrap()
            .wolf_body_armor_dye,
        None,
        "the adult-only equipment identity gate also suppresses its dye"
    );
}

#[test]
fn entity_model_sources_project_named_and_direct_armor_trim_sprites() {
    use std::collections::BTreeMap;

    let iron_helmet = 804;
    let diamond_leggings = 805;
    let mut store = WorldStore::new();
    store.set_item_armor_materials(BTreeMap::from([
        (iron_helmet, ArmorMaterialKind::Iron),
        (diamond_leggings, ArmorMaterialKind::Diamond),
    ]));
    store.apply_add_entity(protocol_add_entity_with_type(
        52,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    ));

    let named = ItemStackSummary {
        item_id: Some(iron_helmet),
        count: 1,
        component_patch: DataComponentPatchSummary {
            armor_trim_material_id: Some(1), // iron
            armor_trim_pattern_id: Some(0),  // sentry
            ..Default::default()
        },
    };
    let direct = ItemStackSummary {
        item_id: Some(diamond_leggings),
        count: 1,
        component_patch: DataComponentPatchSummary {
            armor_trim_material_direct: Some(Box::new(TrimMaterialSummary {
                asset_name: "ruby".to_string(),
                override_armor_assets: BTreeMap::from([(
                    "minecraft:diamond".to_string(),
                    "ruby_darker".to_string(),
                )]),
                description: String::new(),
                description_styled: None,
                description_nbt: None,
            })),
            armor_trim_pattern_direct: Some(Box::new(TrimPatternSummary {
                asset_id: "example:spiral".to_string(),
                description: String::new(),
                description_styled: None,
                description_nbt: None,
                decal: true,
            })),
            ..Default::default()
        },
    };
    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 52,
        slots: vec![
            EquipmentSlotUpdate {
                slot: EquipmentSlot::Head,
                item: named,
            },
            EquipmentSlotUpdate {
                slot: EquipmentSlot::Legs,
                item: direct,
            },
        ],
    }));

    let source = store
        .entity_model_source_at_partial_tick(52, 1.0)
        .expect("trimmed zombie source");
    assert_eq!(
        source.head_armor_trim,
        Some(ArmorTrimSprite {
            sprite_id: "minecraft:trims/entity/humanoid/sentry_iron_darker".to_string(),
            decal: false,
        })
    );
    assert_eq!(
        source.legs_armor_trim,
        Some(ArmorTrimSprite {
            sprite_id: "example:trims/entity/humanoid_leggings/spiral_ruby_darker".to_string(),
            decal: true,
        })
    );
    assert_eq!(source.chest_armor_trim, None);
    assert_eq!(source.feet_armor_trim, None);
}

#[test]
fn entity_model_sources_project_wings_trim_and_foil_from_chest_asset() {
    use std::collections::BTreeMap;

    const ELYTRA_ITEM_ID: i32 = 806;
    const CHESTPLATE_ITEM_ID: i32 = 807;

    fn trimmed_item(item_id: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count: 1,
            component_patch: DataComponentPatchSummary {
                enchantments: vec![ItemEnchantmentSummary {
                    holder_id: 12,
                    level: 1,
                }],
                armor_trim_material_id: Some(1), // iron
                armor_trim_pattern_id: Some(0),  // sentry
                ..Default::default()
            },
        }
    }
    fn wings(store: &WorldStore) -> (Option<ArmorTrimSprite>, bool) {
        let source = store
            .entity_model_source_at_partial_tick(53, 1.0)
            .expect("zombie source");
        (source.chest_wings_trim, source.chest_wings_foil)
    }

    let mut store = WorldStore::new();
    store.set_default_item_wings_equipment_assets(BTreeMap::from([(
        ELYTRA_ITEM_ID,
        "minecraft:elytra".to_string(),
    )]));
    store.apply_add_entity(protocol_add_entity_with_type(
        53,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    ));

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 53,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Chest,
            item: trimmed_item(ELYTRA_ITEM_ID),
        }],
    }));
    assert_eq!(
        wings(&store),
        (
            Some(ArmorTrimSprite {
                sprite_id: "minecraft:trims/entity/wings/sentry_iron".to_string(),
                decal: false,
            }),
            true,
        )
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 53,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Chest,
            item: trimmed_item(CHESTPLATE_ITEM_ID),
        }],
    }));
    assert_eq!(
        wings(&store),
        (None, false),
        "a chest item without a WINGS equipment asset cannot project wings trim or foil"
    );
}

#[test]
fn entity_model_sources_project_synchronized_custom_armor_trim_payloads() {
    use std::collections::BTreeMap;

    fn write_nbt_string(out: &mut Vec<u8>, value: &str) {
        let bytes = value.as_bytes();
        out.extend((bytes.len() as u16).to_be_bytes());
        out.extend(bytes);
    }
    fn write_nbt_string_tag(out: &mut Vec<u8>, name: &str, value: &str) {
        out.push(8);
        write_nbt_string(out, name);
        write_nbt_string(out, value);
    }
    fn material_nbt() -> Vec<u8> {
        let mut out = vec![10];
        write_nbt_string_tag(&mut out, "asset_name", "opal");
        out.push(10);
        write_nbt_string(&mut out, "override_armor_assets");
        write_nbt_string_tag(&mut out, "minecraft:iron", "opal_darker");
        out.push(0);
        write_nbt_string_tag(&mut out, "description", "Opal");
        out.push(0);
        out
    }
    fn pattern_nbt() -> Vec<u8> {
        let mut out = vec![10];
        write_nbt_string_tag(&mut out, "asset_id", "example:spire");
        write_nbt_string_tag(&mut out, "description", "Spire");
        out.push(1);
        write_nbt_string(&mut out, "decal");
        out.push(1);
        out.push(0);
        out
    }

    let iron_helmet = 806;
    let mut store = WorldStore::new();
    store.set_item_armor_materials(BTreeMap::from([(iron_helmet, ArmorMaterialKind::Iron)]));
    store.record_registry_entries(
        crate::TRIM_MATERIAL_REGISTRY,
        0,
        vec![crate::RegistryPacketEntry::with_raw_data(
            "example:opal",
            material_nbt(),
        )],
    );
    store.record_registry_entries(
        crate::TRIM_PATTERN_REGISTRY,
        0,
        vec![crate::RegistryPacketEntry::with_raw_data(
            "example:spire",
            pattern_nbt(),
        )],
    );
    store.apply_add_entity(protocol_add_entity_with_type(
        53,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    ));
    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 53,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Head,
            item: ItemStackSummary {
                item_id: Some(iron_helmet),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    armor_trim_material_id: Some(0),
                    armor_trim_pattern_id: Some(0),
                    ..Default::default()
                },
            },
        }],
    }));

    let source = store
        .entity_model_source_at_partial_tick(53, 1.0)
        .expect("custom trimmed zombie source");
    assert_eq!(
        source.head_armor_trim,
        Some(ArmorTrimSprite {
            sprite_id: "example:trims/entity/humanoid/spire_opal_darker".to_string(),
            decal: true,
        })
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 53,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Head,
            item: ItemStackSummary {
                item_id: Some(iron_helmet),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    armor_trim_material_id: Some(1),
                    armor_trim_pattern_id: Some(0),
                    ..Default::default()
                },
            },
        }],
    }));
    assert_eq!(
        store
            .entity_model_source_at_partial_tick(53, 1.0)
            .unwrap()
            .head_armor_trim,
        None,
        "a synchronized registry owns holder order; an out-of-range id must not fall back to vanilla"
    );
}

#[test]
fn entity_model_sources_project_worn_armor_dye_colors() {
    use std::collections::BTreeMap;
    let leather_chestplate = 810;
    let leather_boots = 811;
    let iron_helmet = 812;
    let dye = 0x3F_6CDA;

    let mut store = WorldStore::new();
    store.set_item_armor_materials(BTreeMap::from([
        (leather_chestplate, ArmorMaterialKind::Leather),
        (leather_boots, ArmorMaterialKind::Leather),
        (iron_helmet, ArmorMaterialKind::Iron),
    ]));
    store.apply_add_entity(protocol_add_entity_with_type(
        51,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    ));

    fn dyed_armor_item(item_id: i32, dye: Option<i32>) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count: 1,
            component_patch: DataComponentPatchSummary {
                dyed_color: dye,
                ..Default::default()
            },
        }
    }

    // A custom-dyed leather chestplate, an undyed leather boot, and an iron helmet (non-dyeable).
    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 51,
        slots: vec![
            EquipmentSlotUpdate {
                slot: EquipmentSlot::Chest,
                item: dyed_armor_item(leather_chestplate, Some(dye)),
            },
            EquipmentSlotUpdate {
                slot: EquipmentSlot::Feet,
                item: dyed_armor_item(leather_boots, None),
            },
            EquipmentSlotUpdate {
                slot: EquipmentSlot::Head,
                item: dyed_armor_item(iron_helmet, None),
            },
        ],
    }));

    let sources = store.entity_model_sources_at_partial_tick(1.0);
    // The dyed leather chestplate carries its `dyed_color`; the undyed leather boot and the bare-of-dye
    // iron helmet carry None (each paired with its resolved material).
    assert_eq!(sources[0].chest_armor, Some(ArmorMaterialKind::Leather));
    assert_eq!(sources[0].chest_armor_dye, Some(dye));
    assert_eq!(sources[0].feet_armor, Some(ArmorMaterialKind::Leather));
    assert_eq!(sources[0].feet_armor_dye, None);
    assert_eq!(sources[0].head_armor, Some(ArmorMaterialKind::Iron));
    assert_eq!(sources[0].head_armor_dye, None);
    assert_eq!(sources[0].legs_armor_dye, None);
}

#[test]
fn entity_model_sources_project_pig_saddle_from_saddle_slot() {
    use crate::ItemEquipmentSlot;
    use std::collections::BTreeMap;

    const SADDLE_ITEM_ID: i32 = 820;
    const PLAIN_ITEM_ID: i32 = 821;

    fn stack(item_id: i32, count: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
        }
    }
    fn pig_saddle(store: &WorldStore, entity_id: i32) -> (bool, Option<ArmorTrimSprite>, bool) {
        let source = store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == entity_id)
            .unwrap();
        (
            source.pig_saddle,
            source.pig_saddle_trim,
            source.pig_saddle_foil,
        )
    }

    let mut store = WorldStore::new();
    store.set_default_item_equipment_slots(BTreeMap::from([(
        SADDLE_ITEM_ID,
        ItemEquipmentSlot::Saddle,
    )]));
    store.apply_add_entity(protocol_add_entity_with_type(
        60,
        VANILLA_ENTITY_TYPE_PIG_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        61,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));

    assert_eq!(pig_saddle(&store, 60), (false, None, false));

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 60,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: ItemStackSummary {
                component_patch: DataComponentPatchSummary {
                    enchantments: vec![ItemEnchantmentSummary {
                        holder_id: 12,
                        level: 1,
                    }],
                    armor_trim_material_id: Some(1), // iron
                    armor_trim_pattern_id: Some(0),  // sentry
                    ..Default::default()
                },
                ..stack(SADDLE_ITEM_ID, 1)
            },
        }],
    }));
    assert_eq!(
        pig_saddle(&store, 60),
        (
            true,
            Some(ArmorTrimSprite {
                sprite_id: "minecraft:trims/entity/pig_saddle/sentry_iron".to_string(),
                decal: false,
            }),
            true,
        )
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 61,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: stack(SADDLE_ITEM_ID, 1),
        }],
    }));
    assert_eq!(pig_saddle(&store, 61), (false, None, false));

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 60,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: stack(PLAIN_ITEM_ID, 1),
        }],
    }));
    assert_eq!(pig_saddle(&store, 60), (false, None, false));

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 60,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: ItemStackSummary::empty(),
        }],
    }));
    assert_eq!(pig_saddle(&store, 60), (false, None, false));
}

#[test]
fn entity_model_sources_project_snow_golem_pumpkin_flag() {
    let (pumpkin_data_id, pumpkin_default, _pumpkin_mask) =
        vanilla_entity_metadata_snow_golem_pumpkin_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID,
        )
        .expect("generated SnowGolem pumpkin accessor must resolve");
    fn pumpkin(store: &WorldStore, entity_id: i32) -> bool {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == entity_id)
            .unwrap()
            .snow_golem_pumpkin
    }

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        70,
        VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        71,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));

    assert!(
        pumpkin(&store, 70),
        "SnowGolem.defineSynchedData defaults DATA_PUMPKIN_ID to the pumpkin mask"
    );
    assert!(!pumpkin(&store, 71));

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 70,
        values: vec![ProtocolEntityDataValue {
            data_id: pumpkin_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(0),
        }],
    }));
    assert!(!pumpkin(&store, 70));

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 70,
        values: vec![ProtocolEntityDataValue {
            data_id: pumpkin_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(pumpkin_default),
        }],
    }));
    assert!(pumpkin(&store, 70));
}

#[test]
fn entity_model_sources_project_equine_saddle_and_ridden_state() {
    use crate::ItemEquipmentSlot;
    use std::collections::BTreeMap;

    const SADDLE_ITEM_ID: i32 = 830;
    const PLAIN_ITEM_ID: i32 = 831;

    fn stack(item_id: i32, count: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
        }
    }
    fn equine_saddle_state(
        store: &WorldStore,
        entity_id: i32,
    ) -> (bool, bool, Option<ArmorTrimSprite>, bool) {
        let source = store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == entity_id)
            .unwrap();
        (
            source.equine_saddle,
            source.equine_saddle_ridden,
            source.equine_saddle_trim,
            source.equine_saddle_foil,
        )
    }
    fn trimmed_saddle(item_id: i32) -> ItemStackSummary {
        ItemStackSummary {
            component_patch: DataComponentPatchSummary {
                enchantments: vec![ItemEnchantmentSummary {
                    holder_id: 12,
                    level: 1,
                }],
                armor_trim_material_id: Some(1), // iron
                armor_trim_pattern_id: Some(0),  // sentry
                ..Default::default()
            },
            ..stack(item_id, 1)
        }
    }

    let mut store = WorldStore::new();
    store.set_default_item_equipment_slots(BTreeMap::from([(
        SADDLE_ITEM_ID,
        ItemEquipmentSlot::Saddle,
    )]));
    store.apply_add_entity(protocol_add_entity_with_type(
        62,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        63,
        VANILLA_ENTITY_TYPE_DONKEY_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        64,
        VANILLA_ENTITY_TYPE_PIG_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        65,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        66,
        VANILLA_ENTITY_TYPE_MULE_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        67,
        VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        68,
        VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID,
    ));

    assert_eq!(equine_saddle_state(&store, 62), (false, false, None, false));

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 62,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: trimmed_saddle(SADDLE_ITEM_ID),
        }],
    }));
    assert_eq!(
        equine_saddle_state(&store, 62),
        (
            true,
            false,
            Some(ArmorTrimSprite {
                sprite_id: "minecraft:trims/entity/horse_saddle/sentry_iron".to_string(),
                decal: false,
            }),
            true,
        )
    );

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 62,
        passenger_ids: vec![65],
    }));
    assert_eq!(
        equine_saddle_state(&store, 62),
        (
            true,
            true,
            Some(ArmorTrimSprite {
                sprite_id: "minecraft:trims/entity/horse_saddle/sentry_iron".to_string(),
                decal: false,
            }),
            true,
        )
    );

    for (entity_id, layer) in [
        (63, "donkey_saddle"),
        (66, "mule_saddle"),
        (67, "skeleton_horse_saddle"),
        (68, "zombie_horse_saddle"),
    ] {
        assert!(store.apply_set_equipment(ProtocolSetEquipment {
            entity_id,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Saddle,
                item: trimmed_saddle(SADDLE_ITEM_ID),
            }],
        }));
        assert_eq!(
            equine_saddle_state(&store, entity_id),
            (
                true,
                false,
                Some(ArmorTrimSprite {
                    sprite_id: format!("minecraft:trims/entity/{layer}/sentry_iron"),
                    decal: false,
                }),
                true,
            )
        );
    }

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 64,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: stack(SADDLE_ITEM_ID, 1),
        }],
    }));
    assert_eq!(
        equine_saddle_state(&store, 64),
        (false, false, None, false),
        "the pig saddle projects through its own render-state flag, not the equine one"
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 62,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: stack(PLAIN_ITEM_ID, 1),
        }],
    }));
    assert_eq!(equine_saddle_state(&store, 62), (false, false, None, false));
}

#[test]
fn entity_model_sources_project_equine_tail_counter() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        66,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        67,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    let animate_tail = |store: &WorldStore, entity_id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == entity_id)
            .unwrap()
            .equine_animate_tail
    };

    // Vanilla `AbstractHorse.aiStep` may start `tailCounter` with `random.nextInt(200) == 0`,
    // and `AbstractHorse.tick` clears it after `++tailCounter > 8`. The exact vanilla client seed
    // is not protocol-visible; bbb uses a deterministic Java LCG seeded by entity id. For entity id
    // 66, the first local `nextInt(200) == 0` occurs on tick 37.
    assert!(!animate_tail(&store, 66));
    store.advance_entity_client_animations(36);
    assert!(!animate_tail(&store, 66));
    store.advance_entity_client_animations(1);
    assert!(animate_tail(&store, 66));
    store.advance_entity_client_animations(6);
    assert!(animate_tail(&store, 66));
    store.advance_entity_client_animations(1);
    assert!(!animate_tail(&store, 66));

    // Non-equines do not allocate or project the equine tail counter.
    store.advance_entity_client_animations(37);
    assert!(!animate_tail(&store, 67));
}

#[test]
fn entity_model_sources_project_equine_pose_animations_from_flags() {
    let accessors = vanilla_entity_metadata_abstract_horse_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_HORSE_ID,
    )
    .expect("generated AbstractHorse metadata accessors must resolve");

    let assert_close = |actual: f32, expected: f32, label: &str| {
        assert!(
            (actual - expected).abs() < 1.0e-6,
            "{label}: expected {expected}, got {actual}"
        );
    };
    let pose = |store: &WorldStore, partial_tick: f32| {
        let source = store
            .entity_model_sources_at_partial_tick(partial_tick)
            .into_iter()
            .find(|source| source.entity_id == 68)
            .expect("horse source");
        (
            source.equine_eat_animation,
            source.equine_stand_animation,
            source.equine_feeding_animation,
        )
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        68,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    assert_eq!(pose(&store, 1.0), (0.0, 0.0, 0.0));

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 68,
        values: vec![protocol_byte_data(
            accessors.flags_data_id,
            accessors.eating_mask | accessors.open_mouth_mask,
        )],
    }));
    assert_eq!(pose(&store, 1.0), (0.0, 0.0, 0.0));
    store.advance_entity_client_animations(1);
    assert_close(pose(&store, 0.5).0, 0.225, "eat partial");
    assert_close(pose(&store, 1.0).0, 0.45, "eat rise");
    assert_eq!(pose(&store, 1.0).1, 0.0);
    assert_close(pose(&store, 1.0).2, 0.75, "mouth rise");

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 68,
        values: vec![protocol_byte_data(
            accessors.flags_data_id,
            accessors.standing_mask | accessors.open_mouth_mask,
        )],
    }));
    store.advance_entity_client_animations(1);
    assert_eq!(
        pose(&store, 1.0).0,
        0.0,
        "vanilla clears eatAnim while standing"
    );
    assert_close(pose(&store, 1.0).1, 0.45, "stand rise");
    assert_close(pose(&store, 1.0).2, 0.975, "mouth second rise");

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 68,
        values: vec![protocol_byte_data(accessors.flags_data_id, 0)],
    }));
    store.advance_entity_client_animations(1);
    assert_close(pose(&store, 1.0).1, 0.17374, "stand cubic falloff");
    assert_close(pose(&store, 1.0).2, 0.2425, "mouth falloff");
}

#[test]
fn entity_model_sources_project_strider_saddle_and_ridden_state() {
    use crate::ItemEquipmentSlot;
    use std::collections::BTreeMap;

    const SADDLE_ITEM_ID: i32 = 832;
    const PLAIN_ITEM_ID: i32 = 833;

    fn stack(item_id: i32, count: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
        }
    }
    fn strider_state(
        store: &WorldStore,
        entity_id: i32,
    ) -> (bool, bool, Option<ArmorTrimSprite>, bool) {
        let source = store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == entity_id)
            .unwrap();
        (
            source.strider_saddle,
            source.strider_ridden,
            source.strider_saddle_trim,
            source.strider_saddle_foil,
        )
    }

    let mut store = WorldStore::new();
    store.set_default_item_equipment_slots(BTreeMap::from([(
        SADDLE_ITEM_ID,
        ItemEquipmentSlot::Saddle,
    )]));
    store.apply_add_entity(protocol_add_entity_with_type(
        66,
        VANILLA_ENTITY_TYPE_STRIDER_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        67,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));

    assert_eq!(strider_state(&store, 66), (false, false, None, false));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 66,
        passenger_ids: vec![67],
    }));
    assert_eq!(
        strider_state(&store, 66),
        (false, true, None, false),
        "strider isRidden is independent of the saddle slot"
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 66,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: ItemStackSummary {
                component_patch: DataComponentPatchSummary {
                    enchantments: vec![ItemEnchantmentSummary {
                        holder_id: 12,
                        level: 1,
                    }],
                    armor_trim_material_id: Some(1), // iron
                    armor_trim_pattern_id: Some(0),  // sentry
                    ..Default::default()
                },
                ..stack(SADDLE_ITEM_ID, 1)
            },
        }],
    }));
    assert_eq!(
        strider_state(&store, 66),
        (
            true,
            true,
            Some(ArmorTrimSprite {
                sprite_id: "minecraft:trims/entity/strider_saddle/sentry_iron".to_string(),
                decal: false,
            }),
            true,
        )
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 67,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: stack(SADDLE_ITEM_ID, 1),
        }],
    }));
    assert_eq!(
        strider_state(&store, 67),
        (false, false, None, false),
        "non-striders do not project the strider saddle flag"
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 66,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: stack(PLAIN_ITEM_ID, 1),
        }],
    }));
    assert_eq!(strider_state(&store, 66), (false, true, None, false));
}

#[test]
fn entity_model_sources_project_camel_saddle_and_ridden_state() {
    use crate::ItemEquipmentSlot;
    use std::collections::BTreeMap;

    const SADDLE_ITEM_ID: i32 = 834;
    const PLAIN_ITEM_ID: i32 = 835;

    fn stack(item_id: i32, count: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
        }
    }
    fn camel_saddle_state(
        store: &WorldStore,
        entity_id: i32,
    ) -> (bool, bool, Option<ArmorTrimSprite>, bool) {
        let source = store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == entity_id)
            .unwrap();
        (
            source.camel_saddle,
            source.camel_saddle_ridden,
            source.camel_saddle_trim,
            source.camel_saddle_foil,
        )
    }

    let mut store = WorldStore::new();
    store.set_default_item_equipment_slots(BTreeMap::from([(
        SADDLE_ITEM_ID,
        ItemEquipmentSlot::Saddle,
    )]));
    store.apply_add_entity(protocol_add_entity_with_type(
        68,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        69,
        VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        70,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));

    assert_eq!(camel_saddle_state(&store, 68), (false, false, None, false));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 68,
        passenger_ids: vec![70],
    }));
    assert_eq!(
        camel_saddle_state(&store, 68),
        (false, false, None, false),
        "the reins gate is only useful when the saddle layer itself renders"
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 68,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: ItemStackSummary {
                component_patch: DataComponentPatchSummary {
                    enchantments: vec![ItemEnchantmentSummary {
                        holder_id: 12,
                        level: 1,
                    }],
                    armor_trim_material_id: Some(1), // iron
                    armor_trim_pattern_id: Some(0),  // sentry
                    ..Default::default()
                },
                ..stack(SADDLE_ITEM_ID, 1)
            },
        }],
    }));
    assert_eq!(
        camel_saddle_state(&store, 68),
        (
            true,
            true,
            Some(ArmorTrimSprite {
                sprite_id: "minecraft:trims/entity/camel_saddle/sentry_iron".to_string(),
                decal: false,
            }),
            true,
        )
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 69,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: ItemStackSummary {
                component_patch: DataComponentPatchSummary {
                    enchantments: vec![ItemEnchantmentSummary {
                        holder_id: 12,
                        level: 1,
                    }],
                    armor_trim_material_id: Some(1), // iron
                    armor_trim_pattern_id: Some(0),  // sentry
                    ..Default::default()
                },
                ..stack(SADDLE_ITEM_ID, 1)
            },
        }],
    }));
    assert_eq!(
        camel_saddle_state(&store, 69),
        (
            true,
            false,
            Some(ArmorTrimSprite {
                sprite_id: "minecraft:trims/entity/camel_husk_saddle/sentry_iron".to_string(),
                decal: false,
            }),
            true,
        )
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 70,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: stack(SADDLE_ITEM_ID, 1),
        }],
    }));
    assert_eq!(
        camel_saddle_state(&store, 70),
        (false, false, None, false),
        "non-camels do not project the camel saddle flag"
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 68,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: stack(PLAIN_ITEM_ID, 1),
        }],
    }));
    assert_eq!(camel_saddle_state(&store, 68), (false, false, None, false));
}

#[test]
fn entity_model_sources_project_nautilus_saddle_from_saddle_slot() {
    use crate::ItemEquipmentSlot;
    use std::collections::BTreeMap;

    const SADDLE_ITEM_ID: i32 = 836;
    const PLAIN_ITEM_ID: i32 = 837;

    fn stack(item_id: i32, count: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
        }
    }
    fn nautilus_saddle(
        store: &WorldStore,
        entity_id: i32,
    ) -> (bool, Option<ArmorTrimSprite>, bool) {
        let source = store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == entity_id)
            .unwrap();
        (
            source.nautilus_saddle,
            source.nautilus_saddle_trim,
            source.nautilus_saddle_foil,
        )
    }
    fn trimmed_saddle(item_id: i32) -> ItemStackSummary {
        ItemStackSummary {
            component_patch: DataComponentPatchSummary {
                enchantments: vec![ItemEnchantmentSummary {
                    holder_id: 12,
                    level: 1,
                }],
                armor_trim_material_id: Some(1), // iron
                armor_trim_pattern_id: Some(0),  // sentry
                ..Default::default()
            },
            ..stack(item_id, 1)
        }
    }

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        71,
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        72,
        VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        73,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 71,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: trimmed_saddle(SADDLE_ITEM_ID),
        }],
    }));
    assert_eq!(
        nautilus_saddle(&store, 71),
        (false, None, false),
        "without the item registry's saddle-slot map, a raw item id is not enough"
    );

    store.set_default_item_equipment_slots(BTreeMap::from([(
        SADDLE_ITEM_ID,
        ItemEquipmentSlot::Saddle,
    )]));
    assert_eq!(
        nautilus_saddle(&store, 71),
        (
            true,
            Some(ArmorTrimSprite {
                sprite_id: "minecraft:trims/entity/nautilus_saddle/sentry_iron".to_string(),
                decal: false,
            }),
            true,
        )
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 72,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: trimmed_saddle(SADDLE_ITEM_ID),
        }],
    }));
    assert_eq!(
        nautilus_saddle(&store, 72),
        (
            true,
            Some(ArmorTrimSprite {
                sprite_id: "minecraft:trims/entity/nautilus_saddle/sentry_iron".to_string(),
                decal: false,
            }),
            true,
        )
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 73,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: stack(SADDLE_ITEM_ID, 1),
        }],
    }));
    assert_eq!(
        nautilus_saddle(&store, 73),
        (false, None, false),
        "non-nautilus entities do not project the nautilus saddle flag"
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 71,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: stack(PLAIN_ITEM_ID, 1),
        }],
    }));
    assert_eq!(nautilus_saddle(&store, 71), (false, None, false));
}

#[test]
fn entity_model_sources_project_nautilus_body_armor_from_body_slot() {
    use std::collections::BTreeMap;

    const IRON_NAUTILUS_ARMOR_ITEM_ID: i32 = 841;
    const NETHERITE_NAUTILUS_ARMOR_ITEM_ID: i32 = 842;
    const PLAIN_ITEM_ID: i32 = 843;

    fn stack(item_id: i32, count: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: DataComponentPatchSummary {
                enchantments: vec![ItemEnchantmentSummary {
                    holder_id: 12,
                    level: 1,
                }],
                armor_trim_material_id: Some(1), // iron
                armor_trim_pattern_id: Some(0),  // sentry
                ..Default::default()
            },
        }
    }
    fn nautilus_body_armor(
        store: &WorldStore,
        entity_id: i32,
    ) -> (Option<ArmorMaterialKind>, Option<ArmorTrimSprite>, bool) {
        let source = store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == entity_id)
            .unwrap();
        (
            source.nautilus_body_armor,
            source.nautilus_body_armor_trim,
            source.nautilus_body_armor_foil,
        )
    }

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        78,
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        79,
        VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        80,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        81,
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 81,
        values: vec![baby_data(VANILLA_ENTITY_TYPE_NAUTILUS_ID, true)],
    }));

    for (entity_id, item_id) in [
        (78, IRON_NAUTILUS_ARMOR_ITEM_ID),
        (79, NETHERITE_NAUTILUS_ARMOR_ITEM_ID),
        (80, IRON_NAUTILUS_ARMOR_ITEM_ID),
        (81, IRON_NAUTILUS_ARMOR_ITEM_ID),
    ] {
        assert!(store.apply_set_equipment(ProtocolSetEquipment {
            entity_id,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Body,
                item: stack(item_id, 1),
            }],
        }));
    }
    assert_eq!(
        nautilus_body_armor(&store, 78),
        (None, None, false),
        "without the item registry's nautilus armor material map, a raw body item id is not enough"
    );

    store.set_default_nautilus_body_armor_materials(BTreeMap::from([
        (IRON_NAUTILUS_ARMOR_ITEM_ID, ArmorMaterialKind::Iron),
        (
            NETHERITE_NAUTILUS_ARMOR_ITEM_ID,
            ArmorMaterialKind::Netherite,
        ),
    ]));
    assert_eq!(
        nautilus_body_armor(&store, 78),
        (
            Some(ArmorMaterialKind::Iron),
            Some(ArmorTrimSprite {
                sprite_id: "minecraft:trims/entity/nautilus_body/sentry_iron_darker".to_string(),
                decal: false,
            }),
            true,
        )
    );
    assert_eq!(
        nautilus_body_armor(&store, 79),
        (
            Some(ArmorMaterialKind::Netherite),
            Some(ArmorTrimSprite {
                sprite_id: "minecraft:trims/entity/nautilus_body/sentry_iron".to_string(),
                decal: false,
            }),
            true,
        )
    );
    assert_eq!(
        nautilus_body_armor(&store, 80),
        (None, None, false),
        "non-nautilus entities do not project the nautilus body armor flag"
    );
    assert_eq!(
        nautilus_body_armor(&store, 81),
        (None, None, false),
        "baby living nautilus skip the layer because vanilla supplies no baby armor model"
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 78,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Body,
            item: stack(PLAIN_ITEM_ID, 1),
        }],
    }));
    assert_eq!(nautilus_body_armor(&store, 78), (None, None, false));
}

#[test]
fn entity_model_sources_project_horse_body_armor_from_body_slot() {
    use std::collections::BTreeMap;

    const LEATHER_HORSE_ARMOR_ITEM_ID: i32 = 844;
    const DIAMOND_HORSE_ARMOR_ITEM_ID: i32 = 845;
    const NETHERITE_HORSE_ARMOR_ITEM_ID: i32 = 846;
    const PLAIN_ITEM_ID: i32 = 847;
    const LEATHER_DYE: i32 = 0x0033_66CC;

    fn stack(item_id: i32, count: i32, dyed_color: Option<i32>) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: DataComponentPatchSummary {
                dyed_color,
                enchantments: vec![ItemEnchantmentSummary {
                    holder_id: 12,
                    level: 1,
                }],
                armor_trim_material_id: Some(1), // iron
                armor_trim_pattern_id: Some(0),  // sentry
                ..Default::default()
            },
        }
    }
    fn horse_body_armor(
        store: &WorldStore,
        entity_id: i32,
    ) -> (
        Option<ArmorMaterialKind>,
        Option<i32>,
        Option<ArmorTrimSprite>,
        bool,
    ) {
        let source = store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == entity_id)
            .unwrap();
        (
            source.equine_body_armor,
            source.equine_body_armor_dye,
            source.equine_body_armor_trim,
            source.equine_body_armor_foil,
        )
    }

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        82,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        83,
        VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        84,
        VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        85,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        86,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 86,
        values: vec![baby_data(VANILLA_ENTITY_TYPE_HORSE_ID, true)],
    }));

    for (entity_id, item_id, dye) in [
        (82, LEATHER_HORSE_ARMOR_ITEM_ID, Some(LEATHER_DYE)),
        (83, NETHERITE_HORSE_ARMOR_ITEM_ID, None),
        (84, DIAMOND_HORSE_ARMOR_ITEM_ID, None),
        (85, LEATHER_HORSE_ARMOR_ITEM_ID, Some(LEATHER_DYE)),
        (86, LEATHER_HORSE_ARMOR_ITEM_ID, Some(LEATHER_DYE)),
    ] {
        assert!(store.apply_set_equipment(ProtocolSetEquipment {
            entity_id,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Body,
                item: stack(item_id, 1, dye),
            }],
        }));
    }
    assert_eq!(
        horse_body_armor(&store, 82),
        (None, None, None, false),
        "without the item registry's horse armor material map, a raw body item id is not enough"
    );

    store.set_default_horse_body_armor_materials(BTreeMap::from([
        (LEATHER_HORSE_ARMOR_ITEM_ID, ArmorMaterialKind::Leather),
        (DIAMOND_HORSE_ARMOR_ITEM_ID, ArmorMaterialKind::Diamond),
        (NETHERITE_HORSE_ARMOR_ITEM_ID, ArmorMaterialKind::Netherite),
    ]));
    assert_eq!(
        horse_body_armor(&store, 82),
        (
            Some(ArmorMaterialKind::Leather),
            Some(LEATHER_DYE),
            Some(ArmorTrimSprite {
                sprite_id: "minecraft:trims/entity/horse_body/sentry_iron".to_string(),
                decal: false,
            }),
            true,
        )
    );
    assert_eq!(
        horse_body_armor(&store, 83),
        (
            Some(ArmorMaterialKind::Netherite),
            None,
            Some(ArmorTrimSprite {
                sprite_id: "minecraft:trims/entity/horse_body/sentry_iron".to_string(),
                decal: false,
            }),
            true,
        )
    );
    assert_eq!(
        horse_body_armor(&store, 84),
        (None, None, None, false),
        "EntityTypeTags.CAN_WEAR_HORSE_ARMOR excludes skeleton horses"
    );
    assert_eq!(
        horse_body_armor(&store, 85),
        (None, None, None, false),
        "non-horse entities do not project horse body armor"
    );
    assert_eq!(
        horse_body_armor(&store, 86),
        (None, None, None, false),
        "baby horses skip the layer because vanilla supplies no baby armor model"
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 82,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Body,
            item: stack(PLAIN_ITEM_ID, 1, Some(LEATHER_DYE)),
        }],
    }));
    assert_eq!(horse_body_armor(&store, 82), (None, None, None, false));
}

#[test]
fn entity_model_sources_project_wolf_body_armor_from_body_slot() {
    use std::collections::BTreeMap;

    const WOLF_ARMOR_ITEM_ID: i32 = 848;
    const PLAIN_ITEM_ID: i32 = 849;
    const WOLF_ARMOR_DYE: i32 = 0x0033_66CC;

    fn stack(
        item_id: i32,
        count: i32,
        dyed_color: Option<i32>,
        damage: Option<i32>,
        unbreakable: bool,
        enchantments: Vec<ItemEnchantmentSummary>,
        enchantment_glint_override: Option<bool>,
    ) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: DataComponentPatchSummary {
                dyed_color,
                damage,
                unbreakable,
                enchantments,
                enchantment_glint_override,
                armor_trim_material_id: Some(1), // iron
                armor_trim_pattern_id: Some(0),  // sentry
                ..Default::default()
            },
        }
    }
    fn wolf_body_armor(
        store: &WorldStore,
        entity_id: i32,
    ) -> (
        Option<ArmorMaterialKind>,
        Option<i32>,
        WolfArmorCrackiness,
        bool,
        Option<ArmorTrimSprite>,
    ) {
        let source = store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == entity_id)
            .unwrap();
        (
            source.wolf_body_armor,
            source.wolf_body_armor_dye,
            source.wolf_body_armor_crackiness,
            source.wolf_body_armor_foil,
            source.wolf_body_armor_trim,
        )
    }

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        87,
        VANILLA_ENTITY_TYPE_WOLF_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        88,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        89,
        VANILLA_ENTITY_TYPE_WOLF_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 89,
        values: vec![baby_data(VANILLA_ENTITY_TYPE_WOLF_ID, true)],
    }));

    for entity_id in [87, 88, 89] {
        assert!(store.apply_set_equipment(ProtocolSetEquipment {
            entity_id,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Body,
                item: stack(
                    WOLF_ARMOR_ITEM_ID,
                    1,
                    Some(WOLF_ARMOR_DYE),
                    Some(4),
                    false,
                    vec![ItemEnchantmentSummary {
                        holder_id: 12,
                        level: 1,
                    }],
                    None,
                ),
            }],
        }));
    }
    assert_eq!(
        wolf_body_armor(&store, 87),
        (None, None, WolfArmorCrackiness::None, false, None),
        "without the item registry's wolf armor material map, a raw body item id is not enough"
    );

    store.set_default_wolf_body_armor_materials(BTreeMap::from([(
        WOLF_ARMOR_ITEM_ID,
        ArmorMaterialKind::ArmadilloScute,
    )]));
    store.set_default_item_max_damage(BTreeMap::from([(WOLF_ARMOR_ITEM_ID, 64)]));
    assert_eq!(
        wolf_body_armor(&store, 87),
        (
            Some(ArmorMaterialKind::ArmadilloScute),
            Some(WOLF_ARMOR_DYE),
            WolfArmorCrackiness::Low,
            true,
            Some(ArmorTrimSprite {
                sprite_id: "minecraft:trims/entity/wolf_body/sentry_iron".to_string(),
                decal: false,
            })
        )
    );
    assert_eq!(
        wolf_body_armor(&store, 88),
        (None, None, WolfArmorCrackiness::None, false, None),
        "non-wolf entities do not project wolf body armor"
    );
    assert_eq!(
        wolf_body_armor(&store, 89),
        (None, None, WolfArmorCrackiness::None, false, None),
        "baby wolves skip WolfArmorLayer because vanilla supplies only the adult WOLF_ARMOR layer"
    );

    for (damage, expected) in [
        (24, WolfArmorCrackiness::Medium),
        (44, WolfArmorCrackiness::High),
    ] {
        assert!(store.apply_set_equipment(ProtocolSetEquipment {
            entity_id: 87,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Body,
                item: stack(
                    WOLF_ARMOR_ITEM_ID,
                    1,
                    None,
                    Some(damage),
                    false,
                    Vec::new(),
                    None
                ),
            }],
        }));
        assert_eq!(wolf_body_armor(&store, 87).2, expected);
    }

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 87,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Body,
            item: stack(
                WOLF_ARMOR_ITEM_ID,
                1,
                None,
                Some(44),
                true,
                vec![ItemEnchantmentSummary {
                    holder_id: 12,
                    level: 1,
                }],
                Some(false),
            ),
        }],
    }));
    assert_eq!(wolf_body_armor(&store, 87).2, WolfArmorCrackiness::None);
    assert!(!wolf_body_armor(&store, 87).3);

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 87,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Body,
            item: stack(
                PLAIN_ITEM_ID,
                0,
                Some(WOLF_ARMOR_DYE),
                Some(44),
                false,
                Vec::new(),
                None,
            ),
        }],
    }));
    assert_eq!(
        wolf_body_armor(&store, 87),
        (None, None, WolfArmorCrackiness::None, false, None)
    );
}

#[test]
fn entity_model_sources_project_llama_body_decor_from_body_slot() {
    use std::collections::BTreeMap;

    const WHITE_CARPET_ITEM_ID: i32 = 838;
    const BLACK_CARPET_ITEM_ID: i32 = 839;
    const PLAIN_ITEM_ID: i32 = 840;

    fn stack(item_id: i32, count: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: DataComponentPatchSummary {
                enchantments: vec![ItemEnchantmentSummary {
                    holder_id: 12,
                    level: 1,
                }],
                armor_trim_material_id: Some(1), // iron
                armor_trim_pattern_id: Some(0),  // sentry
                ..Default::default()
            },
        }
    }
    fn llama_decor(
        store: &WorldStore,
        entity_id: i32,
    ) -> (Option<LlamaBodyDecorColor>, Option<ArmorTrimSprite>, bool) {
        let source = store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == entity_id)
            .unwrap();
        (
            source.llama_body_decor,
            source.llama_body_decor_trim,
            source.llama_body_decor_foil,
        )
    }

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        74,
        VANILLA_ENTITY_TYPE_LLAMA_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        75,
        VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        76,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        77,
        VANILLA_ENTITY_TYPE_LLAMA_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 77,
        values: vec![baby_data(VANILLA_ENTITY_TYPE_LLAMA_ID, true)],
    }));

    for (entity_id, item_id) in [
        (74, WHITE_CARPET_ITEM_ID),
        (75, BLACK_CARPET_ITEM_ID),
        (76, WHITE_CARPET_ITEM_ID),
        (77, WHITE_CARPET_ITEM_ID),
    ] {
        assert!(store.apply_set_equipment(ProtocolSetEquipment {
            entity_id,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Body,
                item: stack(item_id, 1),
            }],
        }));
    }
    assert_eq!(
        llama_decor(&store, 74),
        (None, None, false),
        "without the item registry's llama carpet map, a raw body item id is not enough"
    );

    store.set_default_llama_body_decor_colors(BTreeMap::from([
        (WHITE_CARPET_ITEM_ID, LlamaBodyDecorColor::White),
        (BLACK_CARPET_ITEM_ID, LlamaBodyDecorColor::Black),
    ]));
    let expected_trim = Some(ArmorTrimSprite {
        sprite_id: "minecraft:trims/entity/llama_body/sentry_iron".to_string(),
        decal: false,
    });
    assert_eq!(
        llama_decor(&store, 74),
        (
            Some(LlamaBodyDecorColor::White),
            expected_trim.clone(),
            true
        )
    );
    assert_eq!(
        llama_decor(&store, 75),
        (Some(LlamaBodyDecorColor::Black), expected_trim, true),
        "an explicit carpet wins over the trader-llama empty-stack fallback"
    );
    assert_eq!(
        llama_decor(&store, 76),
        (None, None, false),
        "non-llamas do not project the llama body decor flag"
    );
    assert_eq!(
        llama_decor(&store, 77),
        (None, None, false),
        "baby llamas ignore body items for the decor layer"
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 74,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Body,
            item: stack(PLAIN_ITEM_ID, 1),
        }],
    }));
    assert_eq!(llama_decor(&store, 74), (None, None, false));

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 75,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Body,
            item: stack(BLACK_CARPET_ITEM_ID, 0),
        }],
    }));
    assert_eq!(llama_decor(&store, 75), (None, None, false));
}

#[test]
fn entity_model_sources_project_happy_ghast_harness_leash_holder_and_ridden_state() {
    use std::collections::BTreeMap;

    const WHITE_HARNESS_ITEM_ID: i32 = 841;
    const PLAIN_ITEM_ID: i32 = 842;
    let stack = |item_id, count| ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: DataComponentPatchSummary {
            enchantments: vec![ItemEnchantmentSummary {
                holder_id: 12,
                level: 1,
            }],
            armor_trim_material_id: Some(1), // iron
            armor_trim_pattern_id: Some(0),  // sentry
            ..Default::default()
        },
    };
    let state = |store: &WorldStore, entity_id| {
        let source = store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == entity_id)
            .unwrap();
        (
            source.happy_ghast_body_item,
            source.happy_ghast_harness,
            source.happy_ghast_harness_trim,
            source.happy_ghast_harness_foil,
            source.happy_ghast_body_item_is_harness,
            source.happy_ghast_is_leash_holder,
            source.happy_ghast_ridden,
        )
    };

    let mut store = WorldStore::new();
    store.apply_update_tags(bbb_protocol::packets::UpdateTags {
        registries: vec![bbb_protocol::packets::RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![bbb_protocol::packets::TagNetworkPayload {
                tag: "minecraft:harnesses".to_string(),
                entries: vec![WHITE_HARNESS_ITEM_ID],
            }],
        }],
    });
    store.apply_add_entity(protocol_add_entity_with_type(
        88,
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        89,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        90,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    for entity_id in [88, 89] {
        assert!(store.apply_set_equipment(ProtocolSetEquipment {
            entity_id,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Body,
                item: stack(PLAIN_ITEM_ID, 1),
            }],
        }));
    }
    assert_eq!(
        state(&store, 88),
        (true, None, None, false, false, false, false)
    );
    assert_eq!(
        state(&store, 89),
        (false, None, None, false, false, false, false)
    );

    store.set_default_happy_ghast_harness_colors(BTreeMap::from([(
        WHITE_HARNESS_ITEM_ID,
        HappyGhastHarnessColor::White,
    )]));
    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 88,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Body,
            item: stack(WHITE_HARNESS_ITEM_ID, 1),
        }],
    }));
    assert_eq!(
        state(&store, 88),
        (
            true,
            Some(HappyGhastHarnessColor::White),
            Some(ArmorTrimSprite {
                sprite_id: "minecraft:trims/entity/happy_ghast_body/sentry_iron".to_string(),
                decal: false,
            }),
            true,
            true,
            false,
            false,
        )
    );
    let accessors = vanilla_entity_metadata_happy_ghast_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
    )
    .unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 88,
        values: vec![ProtocolEntityDataValue {
            data_id: accessors.is_leash_holder_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    assert!(state(&store, 88).5);
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 88,
        passenger_ids: vec![90],
    }));
    assert!(state(&store, 88).6);

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 88,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Body,
            item: stack(WHITE_HARNESS_ITEM_ID, 0),
        }],
    }));
    assert_eq!(
        state(&store, 88),
        (false, None, None, false, false, true, true)
    );
}

#[test]
fn entity_model_sources_project_in_water_from_world_fluid() {
    // Vanilla `LivingEntityRenderState.isInWater = entity.isInWater()`: the scene projects
    // the `wasTouchingWater` overlap of the entity's world AABB against the chunk fluid
    // state. A cod (0.5 × 0.3 box) and a horse (whose `AbstractEquineModel.setupAnim`
    // slows the leg phase in water) submerged in source water are in water; the same
    // entities in air are not.
    const SOURCE_WATER_BLOCK_STATE_ID: i32 = 86;
    let source_by_id = |store: &WorldStore, entity_id| {
        store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == entity_id)
            .expect("entity source")
    };

    let mut store = WorldStore::with_dimension(crate::WorldDimension {
        min_y: 0,
        height: 16,
    });
    store.insert_decoded_chunk(empty_test_chunk());
    store.apply_add_entity(ProtocolAddEntity {
        id: 50,
        uuid: default_entity_uuid(),
        entity_type_id: VANILLA_ENTITY_TYPE_COD_ID,
        position: ProtocolVec3d {
            x: 8.5,
            y: 2.0,
            z: 8.5,
        },
        delta_movement: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 99,
    });
    store.apply_add_entity(ProtocolAddEntity {
        id: 52,
        uuid: default_entity_uuid(),
        entity_type_id: VANILLA_ENTITY_TYPE_HORSE_ID,
        position: ProtocolVec3d {
            x: 8.5,
            y: 2.0,
            z: 10.5,
        },
        delta_movement: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 99,
    });

    assert!(
        !source_by_id(&store, 50).in_water,
        "a cod in air is not in water"
    );
    assert!(
        !source_by_id(&store, 52).in_water,
        "a horse in air is not in water"
    );

    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos { x: 8, y: 2, z: 8 },
        block_state_id: SOURCE_WATER_BLOCK_STATE_ID,
    }));
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos { x: 8, y: 2, z: 10 },
        block_state_id: SOURCE_WATER_BLOCK_STATE_ID,
    }));
    let wet_cod = source_by_id(&store, 50);
    let wet_horse = source_by_id(&store, 52);
    assert!(wet_cod.in_water, "a cod inside a water column is in water");
    assert!(
        wet_horse.in_water,
        "a horse inside a water column is in water"
    );
    assert!(
        !wet_cod.boat_underwater && !wet_horse.boat_underwater,
        "non-boat entities keep the boat-only underwater flag unset"
    );
}

#[test]
fn entity_model_sources_project_boat_underwater_from_top_fluid() {
    // Vanilla `AbstractBoat.isUnderWater()` is a top-slice test: bottom contact with
    // water is not enough, but water whose surface is above the boat AABB top sets
    // `BoatRenderState.isUnderWater` for bubble and water-mask gating.
    const SOURCE_WATER_BLOCK_STATE_ID: i32 = 86;

    let mut store = WorldStore::with_dimension(crate::WorldDimension {
        min_y: 0,
        height: 16,
    });
    store.insert_decoded_chunk(empty_test_chunk());
    store.apply_add_entity(ProtocolAddEntity {
        id: 51,
        uuid: default_entity_uuid(),
        entity_type_id: VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        position: ProtocolVec3d {
            x: 8.5,
            y: 2.0,
            z: 8.5,
        },
        delta_movement: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 99,
    });

    let underwater = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == 51)
            .expect("boat source")
            .boat_underwater
    };

    assert!(!underwater(&store), "a boat in air is not underwater");
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos { x: 8, y: 1, z: 8 },
        block_state_id: SOURCE_WATER_BLOCK_STATE_ID,
    }));
    assert!(
        !underwater(&store),
        "water below the boat top does not satisfy AbstractBoat.isUnderWater"
    );
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos { x: 8, y: 2, z: 8 },
        block_state_id: SOURCE_WATER_BLOCK_STATE_ID,
    }));
    assert!(
        underwater(&store),
        "a source water surface above the boat top sets BoatRenderState.isUnderWater"
    );
}
