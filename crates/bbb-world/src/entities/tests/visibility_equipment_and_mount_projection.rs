use super::*;

#[test]
fn entity_model_sources_project_boat_bubble_angle_from_bubble_time() {
    const BOAT_BUBBLE_TIME_DATA_ID: u8 = 13;

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
        values: vec![protocol_int_data(BOAT_BUBBLE_TIME_DATA_ID, 60)],
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
        values: vec![protocol_int_data(BOAT_BUBBLE_TIME_DATA_ID, 0)],
    }));
    store.advance_entity_client_animations(1);
    assert_close(bubble(&store, 1.0), 0.0, "cleared bubble angle");
}

#[test]
fn entity_model_sources_project_invisible_to_player_for_spectator_viewer() {
    const ENTITY_SHARED_FLAGS_DATA_ID: u8 = 0;
    const ENTITY_SHARED_FLAG_INVISIBLE: i8 = 1 << 5;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        35,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 35,
        values: vec![protocol_byte_data(
            ENTITY_SHARED_FLAGS_DATA_ID,
            ENTITY_SHARED_FLAG_INVISIBLE,
        )],
    }));
    store.apply_add_entity(protocol_add_entity_with_type(
        36,
        VANILLA_ENTITY_TYPE_MINECART_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 36,
        values: vec![protocol_byte_data(
            ENTITY_SHARED_FLAGS_DATA_ID,
            ENTITY_SHARED_FLAG_INVISIBLE,
        )],
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
    const ENTITY_SHARED_FLAGS_DATA_ID: u8 = 0;
    const ENTITY_SHARED_FLAG_INVISIBLE: i8 = 1 << 5;

    let mut store = WorldStore::new();
    let local_uuid = Uuid::from_u128(0x33345678123456781234567812345678);
    let chicken_uuid = Uuid::from_u128(0x44345678123456781234567812345678);
    let minecart_uuid = Uuid::from_u128(0x55345678123456781234567812345678);

    let mut local_player = protocol_add_entity_with_type(40, VANILLA_ENTITY_TYPE_PLAYER_ID);
    local_player.uuid = local_uuid;
    store.apply_add_entity(local_player);
    store.apply_login(&protocol_play_login(40));
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
            values: vec![protocol_byte_data(
                ENTITY_SHARED_FLAGS_DATA_ID,
                ENTITY_SHARED_FLAG_INVISIBLE,
            )],
        }));
    }

    assert!(store.apply_set_player_team(ProtocolSetPlayerTeam {
        name: "green".to_string(),
        method: ProtocolPlayerTeamMethod::Add,
        parameters: Some(ProtocolPlayerTeamParameters {
            display_name: "Green".to_string(),
            options: 0,
            nametag_visibility: ProtocolTeamVisibility::Always,
            collision_rule: ProtocolTeamCollisionRule::Always,
            color: ProtocolChatFormatting::Green,
            player_prefix: String::new(),
            player_suffix: String::new(),
        }),
        players: vec![
            "PickTarget".to_string(),
            chicken_uuid.to_string(),
            minecart_uuid.to_string(),
        ],
    }));

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
        method: ProtocolPlayerTeamMethod::Change,
        parameters: Some(ProtocolPlayerTeamParameters {
            display_name: "Green".to_string(),
            options: 2,
            nametag_visibility: ProtocolTeamVisibility::Always,
            collision_rule: ProtocolTeamCollisionRule::Always,
            color: ProtocolChatFormatting::Green,
            player_prefix: String::new(),
            player_suffix: String::new(),
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
    const ENTITY_SHARED_FLAGS_DATA_ID: u8 = 0;
    const ENTITY_SHARED_FLAG_GLOWING: i8 = 1 << 6;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        37,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 37,
        values: vec![protocol_byte_data(
            ENTITY_SHARED_FLAGS_DATA_ID,
            ENTITY_SHARED_FLAG_GLOWING,
        )],
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
    const ENTITY_SHARED_FLAGS_DATA_ID: u8 = 0;
    const ENTITY_SHARED_FLAG_GLOWING: i8 = 1 << 6;

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
        method: ProtocolPlayerTeamMethod::Add,
        parameters: Some(ProtocolPlayerTeamParameters {
            display_name: "Green".to_string(),
            options: 0,
            nametag_visibility: ProtocolTeamVisibility::Always,
            collision_rule: ProtocolTeamCollisionRule::Always,
            color: ProtocolChatFormatting::Green,
            player_prefix: String::new(),
            player_suffix: String::new(),
        }),
        players: vec![chicken_uuid.to_string(), "PickTarget".to_string()],
    }));
    for id in [37, 38] {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![protocol_byte_data(
                ENTITY_SHARED_FLAGS_DATA_ID,
                ENTITY_SHARED_FLAG_GLOWING,
            )],
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
    fn pig_saddle(store: &WorldStore, entity_id: i32) -> bool {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == entity_id)
            .unwrap()
            .pig_saddle
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

    assert!(!pig_saddle(&store, 60));

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 60,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: stack(SADDLE_ITEM_ID, 1),
        }],
    }));
    assert!(pig_saddle(&store, 60));

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 61,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: stack(SADDLE_ITEM_ID, 1),
        }],
    }));
    assert!(!pig_saddle(&store, 61));

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 60,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: stack(PLAIN_ITEM_ID, 1),
        }],
    }));
    assert!(!pig_saddle(&store, 60));

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 60,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: ItemStackSummary::empty(),
        }],
    }));
    assert!(!pig_saddle(&store, 60));
}

#[test]
fn entity_model_sources_project_snow_golem_pumpkin_flag() {
    const SNOW_GOLEM_PUMPKIN_DATA_ID: u8 = 16;

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
        "SnowGolem.defineSynchedData defaults DATA_PUMPKIN_ID to bit 16"
    );
    assert!(!pumpkin(&store, 71));

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 70,
        values: vec![ProtocolEntityDataValue {
            data_id: SNOW_GOLEM_PUMPKIN_DATA_ID,
            serializer_id: 0,
            value: EntityDataValueKind::Byte(0),
        }],
    }));
    assert!(!pumpkin(&store, 70));

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 70,
        values: vec![ProtocolEntityDataValue {
            data_id: SNOW_GOLEM_PUMPKIN_DATA_ID,
            serializer_id: 0,
            value: EntityDataValueKind::Byte(16),
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
    fn equine_saddle_state(store: &WorldStore, entity_id: i32) -> (bool, bool) {
        let source = store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == entity_id)
            .unwrap();
        (source.equine_saddle, source.equine_saddle_ridden)
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

    assert_eq!(equine_saddle_state(&store, 62), (false, false));

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 62,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: stack(SADDLE_ITEM_ID, 1),
        }],
    }));
    assert_eq!(equine_saddle_state(&store, 62), (true, false));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 62,
        passenger_ids: vec![65],
    }));
    assert_eq!(equine_saddle_state(&store, 62), (true, true));

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 63,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: stack(SADDLE_ITEM_ID, 1),
        }],
    }));
    assert_eq!(equine_saddle_state(&store, 63), (true, false));

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 64,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: stack(SADDLE_ITEM_ID, 1),
        }],
    }));
    assert_eq!(
        equine_saddle_state(&store, 64),
        (false, false),
        "the pig saddle projects through its own render-state flag, not the equine one"
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 62,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: stack(PLAIN_ITEM_ID, 1),
        }],
    }));
    assert_eq!(equine_saddle_state(&store, 62), (false, false));
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
    const ABSTRACT_HORSE_FLAGS_DATA_ID: u8 = 18;
    const FLAG_EATING: i8 = 0x10;
    const FLAG_STANDING: i8 = 0x20;
    const FLAG_OPEN_MOUTH: i8 = 0x40;

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
            ABSTRACT_HORSE_FLAGS_DATA_ID,
            FLAG_EATING | FLAG_OPEN_MOUTH,
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
            ABSTRACT_HORSE_FLAGS_DATA_ID,
            FLAG_STANDING | FLAG_OPEN_MOUTH,
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
        values: vec![protocol_byte_data(ABSTRACT_HORSE_FLAGS_DATA_ID, 0)],
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
    fn strider_state(store: &WorldStore, entity_id: i32) -> (bool, bool) {
        let source = store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == entity_id)
            .unwrap();
        (source.strider_saddle, source.strider_ridden)
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

    assert_eq!(strider_state(&store, 66), (false, false));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 66,
        passenger_ids: vec![67],
    }));
    assert_eq!(
        strider_state(&store, 66),
        (false, true),
        "strider isRidden is independent of the saddle slot"
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 66,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: stack(SADDLE_ITEM_ID, 1),
        }],
    }));
    assert_eq!(strider_state(&store, 66), (true, true));

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 67,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: stack(SADDLE_ITEM_ID, 1),
        }],
    }));
    assert_eq!(
        strider_state(&store, 67),
        (false, false),
        "non-striders do not project the strider saddle flag"
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 66,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: stack(PLAIN_ITEM_ID, 1),
        }],
    }));
    assert_eq!(strider_state(&store, 66), (false, true));
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
    fn camel_saddle_state(store: &WorldStore, entity_id: i32) -> (bool, bool) {
        let source = store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == entity_id)
            .unwrap();
        (source.camel_saddle, source.camel_saddle_ridden)
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

    assert_eq!(camel_saddle_state(&store, 68), (false, false));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 68,
        passenger_ids: vec![70],
    }));
    assert_eq!(
        camel_saddle_state(&store, 68),
        (false, false),
        "the reins gate is only useful when the saddle layer itself renders"
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 68,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: stack(SADDLE_ITEM_ID, 1),
        }],
    }));
    assert_eq!(camel_saddle_state(&store, 68), (true, true));

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 69,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: stack(SADDLE_ITEM_ID, 1),
        }],
    }));
    assert_eq!(camel_saddle_state(&store, 69), (true, false));

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 70,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: stack(SADDLE_ITEM_ID, 1),
        }],
    }));
    assert_eq!(
        camel_saddle_state(&store, 70),
        (false, false),
        "non-camels do not project the camel saddle flag"
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 68,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: stack(PLAIN_ITEM_ID, 1),
        }],
    }));
    assert_eq!(camel_saddle_state(&store, 68), (false, false));
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
    fn nautilus_saddle(store: &WorldStore, entity_id: i32) -> bool {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == entity_id)
            .unwrap()
            .nautilus_saddle
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
            item: stack(SADDLE_ITEM_ID, 1),
        }],
    }));
    assert!(
        !nautilus_saddle(&store, 71),
        "without the item registry's saddle-slot map, a raw item id is not enough"
    );

    store.set_default_item_equipment_slots(BTreeMap::from([(
        SADDLE_ITEM_ID,
        ItemEquipmentSlot::Saddle,
    )]));
    assert!(nautilus_saddle(&store, 71));

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 72,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: stack(SADDLE_ITEM_ID, 1),
        }],
    }));
    assert!(nautilus_saddle(&store, 72));

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 73,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: stack(SADDLE_ITEM_ID, 1),
        }],
    }));
    assert!(
        !nautilus_saddle(&store, 73),
        "non-nautilus entities do not project the nautilus saddle flag"
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 71,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: stack(PLAIN_ITEM_ID, 1),
        }],
    }));
    assert!(!nautilus_saddle(&store, 71));
}

#[test]
fn entity_model_sources_project_nautilus_body_armor_from_body_slot() {
    use std::collections::BTreeMap;

    const IRON_NAUTILUS_ARMOR_ITEM_ID: i32 = 841;
    const NETHERITE_NAUTILUS_ARMOR_ITEM_ID: i32 = 842;
    const PLAIN_ITEM_ID: i32 = 843;
    const AGEABLE_BABY_DATA_ID: u8 = 16;

    fn stack(item_id: i32, count: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
        }
    }
    fn nautilus_body_armor(store: &WorldStore, entity_id: i32) -> Option<ArmorMaterialKind> {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == entity_id)
            .unwrap()
            .nautilus_body_armor
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
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
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
        None,
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
        Some(ArmorMaterialKind::Iron)
    );
    assert_eq!(
        nautilus_body_armor(&store, 79),
        Some(ArmorMaterialKind::Netherite)
    );
    assert_eq!(
        nautilus_body_armor(&store, 80),
        None,
        "non-nautilus entities do not project the nautilus body armor flag"
    );
    assert_eq!(
        nautilus_body_armor(&store, 81),
        None,
        "baby living nautilus skip the layer because vanilla supplies no baby armor model"
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 78,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Body,
            item: stack(PLAIN_ITEM_ID, 1),
        }],
    }));
    assert_eq!(nautilus_body_armor(&store, 78), None);
}

#[test]
fn entity_model_sources_project_horse_body_armor_from_body_slot() {
    use std::collections::BTreeMap;

    const LEATHER_HORSE_ARMOR_ITEM_ID: i32 = 844;
    const DIAMOND_HORSE_ARMOR_ITEM_ID: i32 = 845;
    const NETHERITE_HORSE_ARMOR_ITEM_ID: i32 = 846;
    const PLAIN_ITEM_ID: i32 = 847;
    const AGEABLE_BABY_DATA_ID: u8 = 16;
    const LEATHER_DYE: i32 = 0x0033_66CC;

    fn stack(item_id: i32, count: i32, dyed_color: Option<i32>) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: DataComponentPatchSummary {
                dyed_color,
                ..Default::default()
            },
        }
    }
    fn horse_body_armor(
        store: &WorldStore,
        entity_id: i32,
    ) -> (Option<ArmorMaterialKind>, Option<i32>) {
        let source = store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == entity_id)
            .unwrap();
        (source.equine_body_armor, source.equine_body_armor_dye)
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
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
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
        (None, None),
        "without the item registry's horse armor material map, a raw body item id is not enough"
    );

    store.set_default_horse_body_armor_materials(BTreeMap::from([
        (LEATHER_HORSE_ARMOR_ITEM_ID, ArmorMaterialKind::Leather),
        (DIAMOND_HORSE_ARMOR_ITEM_ID, ArmorMaterialKind::Diamond),
        (NETHERITE_HORSE_ARMOR_ITEM_ID, ArmorMaterialKind::Netherite),
    ]));
    assert_eq!(
        horse_body_armor(&store, 82),
        (Some(ArmorMaterialKind::Leather), Some(LEATHER_DYE))
    );
    assert_eq!(
        horse_body_armor(&store, 83),
        (Some(ArmorMaterialKind::Netherite), None)
    );
    assert_eq!(
        horse_body_armor(&store, 84),
        (None, None),
        "EntityTypeTags.CAN_WEAR_HORSE_ARMOR excludes skeleton horses"
    );
    assert_eq!(
        horse_body_armor(&store, 85),
        (None, None),
        "non-horse entities do not project horse body armor"
    );
    assert_eq!(
        horse_body_armor(&store, 86),
        (None, None),
        "baby horses skip the layer because vanilla supplies no baby armor model"
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 82,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Body,
            item: stack(PLAIN_ITEM_ID, 1, Some(LEATHER_DYE)),
        }],
    }));
    assert_eq!(horse_body_armor(&store, 82), (None, None));
}

#[test]
fn entity_model_sources_project_wolf_body_armor_from_body_slot() {
    use std::collections::BTreeMap;

    const WOLF_ARMOR_ITEM_ID: i32 = 848;
    const PLAIN_ITEM_ID: i32 = 849;
    const AGEABLE_BABY_DATA_ID: u8 = 16;
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
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
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
        (None, None, WolfArmorCrackiness::None, false),
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
            true
        )
    );
    assert_eq!(
        wolf_body_armor(&store, 88),
        (None, None, WolfArmorCrackiness::None, false),
        "non-wolf entities do not project wolf body armor"
    );
    assert_eq!(
        wolf_body_armor(&store, 89),
        (None, None, WolfArmorCrackiness::None, false),
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
        (None, None, WolfArmorCrackiness::None, false)
    );
}

#[test]
fn entity_model_sources_project_llama_body_decor_from_body_slot() {
    use std::collections::BTreeMap;

    const WHITE_CARPET_ITEM_ID: i32 = 838;
    const BLACK_CARPET_ITEM_ID: i32 = 839;
    const PLAIN_ITEM_ID: i32 = 840;
    const AGEABLE_BABY_DATA_ID: u8 = 16;

    fn stack(item_id: i32, count: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
        }
    }
    fn llama_decor(store: &WorldStore, entity_id: i32) -> Option<LlamaBodyDecorColor> {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == entity_id)
            .unwrap()
            .llama_body_decor
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
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
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
        None,
        "without the item registry's llama carpet map, a raw body item id is not enough"
    );

    store.set_default_llama_body_decor_colors(BTreeMap::from([
        (WHITE_CARPET_ITEM_ID, LlamaBodyDecorColor::White),
        (BLACK_CARPET_ITEM_ID, LlamaBodyDecorColor::Black),
    ]));
    assert_eq!(llama_decor(&store, 74), Some(LlamaBodyDecorColor::White));
    assert_eq!(llama_decor(&store, 75), Some(LlamaBodyDecorColor::Black));
    assert_eq!(
        llama_decor(&store, 76),
        None,
        "non-llamas do not project the llama body decor flag"
    );
    assert_eq!(
        llama_decor(&store, 77),
        None,
        "baby llamas ignore body items for the decor layer"
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 74,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Body,
            item: stack(PLAIN_ITEM_ID, 1),
        }],
    }));
    assert_eq!(llama_decor(&store, 74), None);

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 75,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Body,
            item: stack(BLACK_CARPET_ITEM_ID, 0),
        }],
    }));
    assert_eq!(llama_decor(&store, 75), None);
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
