use super::*;
use crate::item_profiles::test_equippable;
use std::collections::BTreeMap;

fn equippable_asset(slot: EquipmentSlot, asset: &str) -> EquippableSummary {
    EquippableSummary {
        asset_id: Some(asset.to_string()),
        ..test_equippable(slot)
    }
}

fn render_source(store: &WorldStore, entity_id: i32, partial_tick: f32) -> EntityRenderSource {
    store
        .entity_model_source_at_partial_tick(entity_id, partial_tick)
        .expect("entity render source")
}

fn equip_slots(
    store: &mut WorldStore,
    entity_id: i32,
    slots: impl IntoIterator<Item = (EquipmentSlot, ItemStackSummary)>,
) {
    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id,
        slots: slots
            .into_iter()
            .map(|(slot, item)| EquipmentSlotUpdate { slot, item })
            .collect(),
    }));
}

fn equip_slot(store: &mut WorldStore, entity_id: i32, slot: EquipmentSlot, item: ItemStackSummary) {
    equip_slots(store, entity_id, [(slot, item)]);
}

fn set_entity_data(
    store: &mut WorldStore,
    id: i32,
    values: impl IntoIterator<Item = ProtocolEntityDataValue>,
) {
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id,
        values: values.into_iter().collect(),
    }));
}

fn set_passengers(store: &mut WorldStore, vehicle_id: i32, passenger_ids: &[i32]) {
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id,
        passenger_ids: passenger_ids.to_vec(),
    }));
}

fn configure_equipment_assets(
    store: &mut WorldStore,
    generation: u64,
    names: &[&str],
    defaults: &[(i32, EquipmentSlot, &str)],
) {
    store.items.equipment_asset_generation = generation;
    store.items.equipment_asset_names = names
        .iter()
        .map(|s| (*s).to_string())
        .collect::<Vec<_>>()
        .into();
    store.set_test_equippable_defaults(
        defaults
            .iter()
            .map(|&(item_id, slot, asset)| (item_id, equippable_asset(slot, asset))),
    );
}

fn plain_stack(item_id: i32, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    }
}

fn sentry_iron_trimmed_stack(
    item_id: i32,
    count: i32,
    dyed_color: Option<i32>,
) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: DataComponentPatchSummary {
            dyed_color,
            enchantments: vec![ItemEnchantmentSummary {
                holder_id: 12,
                level: 1,
            }],
            armor_trim_material_id: Some(5), // iron in loader order
            armor_trim_pattern_id: Some(8),  // sentry in loader order
            ..Default::default()
        }
        .into(),
    }
}

fn trim_sprite(sprite_id: &str) -> Option<ArmorTrimSprite> {
    Some(ArmorTrimSprite {
        sprite_id: sprite_id.to_string(),
        decal: false,
    })
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

    equip_slot(
        &mut store,
        10,
        EquipmentSlot::MainHand,
        plain_stack(bbb_protocol::item_types::VANILLA_ITEM_FISHING_ROD_ID, 1),
    );
    let accessors = vanilla_entity_metadata_entity_accessors().unwrap();
    set_entity_data(
        &mut store,
        10,
        [protocol_pose_data(
            accessors.pose_data_id,
            bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROUCHING_ID,
        )],
    );
    let source = store
        .fishing_hook_render_source_at_partial_tick(20, 1.0)
        .unwrap();
    assert!(!source.holding_arm_left);
    assert!(source.owner_is_crouching);
    assert!(store
        .fishing_hook_render_source_at_partial_tick(10, 1.0)
        .is_none());
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
    let bubble = |store: &WorldStore, partial_tick: f32| {
        render_source(store, 22, partial_tick)
            .render
            .kind
            .boat()
            .bubble_angle
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        22,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert_eq!(bubble(&store, 1.0), 0.0);

    set_entity_data(
        &mut store,
        22,
        [protocol_int_data(accessors.bubble_time_data_id, 60)],
    );
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

    set_entity_data(
        &mut store,
        22,
        [protocol_int_data(
            accessors.bubble_time_data_id,
            accessors.bubble_time_default,
        )],
    );
    store.advance_entity_client_animations(1);
    assert_close(bubble(&store, 1.0), 0.0, "cleared bubble angle");
}

#[test]
fn entity_model_sources_project_invisible_to_player_for_spectator_viewer() {
    let invisible_mask = entity_metadata_accessors().invisible_mask;
    let mut store = WorldStore::new();
    for (id, entity_type) in [
        (35, VANILLA_ENTITY_TYPE_CHICKEN_ID),
        (36, VANILLA_ENTITY_TYPE_MINECART_ID),
    ] {
        store.apply_add_entity(protocol_add_entity_with_type(id, entity_type));
        set_entity_data(&mut store, id, [entity_shared_flags_data(invisible_mask)]);
    }

    assert!(render_source(&store, 35, 1.0).render.invisible_to_player);
    assert!(render_source(&store, 36, 1.0).render.invisible_to_player);

    store.apply_game_event(ProtocolGameEvent {
        event_id: 3,
        param: 3.0,
    });
    assert!(!render_source(&store, 35, 1.0).render.invisible_to_player);
    assert!(render_source(&store, 36, 1.0).render.invisible_to_player);
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

    for (id, entity_type, uuid) in [
        (41, VANILLA_ENTITY_TYPE_CHICKEN_ID, chicken_uuid),
        (42, VANILLA_ENTITY_TYPE_MINECART_ID, minecart_uuid),
    ] {
        let mut entity = protocol_add_entity_with_type(id, entity_type);
        entity.uuid = uuid;
        store.apply_add_entity(entity);
        set_entity_data(&mut store, id, [entity_shared_flags_data(invisible_mask)]);
    }

    let team_params = |options: u8| ProtocolPlayerTeamParameters {
        display_name: bbb_protocol::RichTextComponent::plain("Green"),
        options,
        nametag_visibility: ProtocolTeamVisibility::Always,
        collision_rule: ProtocolTeamCollisionRule::Always,
        color: ProtocolChatFormatting::Green,
        player_prefix: bbb_protocol::RichTextComponent::plain(""),
        player_suffix: bbb_protocol::RichTextComponent::plain(""),
    };
    assert!(store.apply_set_player_team(ProtocolSetPlayerTeam {
        name: "green".into(),
        method: ProtocolPlayerTeamMethod::Add.into(),
        parameters: Some(team_params(0)),
        players: vec![
            "PickTarget".into(),
            chicken_uuid.to_string().into(),
            minecart_uuid.to_string().into(),
        ],
    }));
    assert_eq!(store.local_player_team_color_rgb(), Some(0x55ff55));
    assert!(render_source(&store, 41, 1.0).render.invisible_to_player);

    assert!(store.apply_set_player_team(ProtocolSetPlayerTeam {
        name: "green".into(),
        method: ProtocolPlayerTeamMethod::Change.into(),
        parameters: Some(team_params(2)),
        players: Vec::new(),
    }));
    let friendly_chicken = render_source(&store, 41, 1.0);
    let friendly_minecart = render_source(&store, 42, 1.0);
    assert!(friendly_chicken.render.invisible);
    assert!(friendly_minecart.render.invisible);
    assert!(!friendly_chicken.render.invisible_to_player);
    assert!(friendly_minecart.render.invisible_to_player);
}

#[test]
fn entity_model_sources_project_glowing_shared_flag() {
    let glowing_mask = entity_metadata_accessors().glowing_mask;
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        37,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    set_entity_data(&mut store, 37, [entity_shared_flags_data(glowing_mask)]);

    let chicken = render_source(&store, 37, 1.0);
    assert!(chicken.render.appears_glowing);
    assert_eq!(chicken.render.outline_color, 0xffff_ffff);
    assert!(!chicken.render.invisible_to_player);
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
        name: "green".into(),
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
        players: vec![chicken_uuid.to_string().into(), "PickTarget".into()],
    }));
    for id in [37, 38] {
        set_entity_data(&mut store, id, [entity_shared_flags_data(glowing_mask)]);
    }

    for id in [37, 38] {
        let source = render_source(&store, id, 1.0);
        assert!(source.render.appears_glowing);
        assert_eq!(source.render.outline_color, 0xff55_ff55);
        assert_eq!(store.entity_team_color_rgb(id), Some(0x55ff55));
    }
    assert_eq!(store.entity_team_color_rgb(999), None);
}

#[test]
fn spectator_outline_key_glows_every_player_for_a_spectating_local_player() {
    // Vanilla `Minecraft.shouldEntityAppearGlowing`: a spectating local player
    // holding `key.spectatorOutlines` outlines players (and only players)
    // without the synced glowing flag.
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(9));
    store.apply_add_entity(protocol_add_entity_with_type(
        37,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        38,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    store.apply_game_event(bbb_protocol::packets::GameEvent {
        event_id: 3,
        param: 3.0,
    });
    store.set_spectator_outlines_key_down(true);

    let chicken = render_source(&store, 37, 1.0);
    let player = render_source(&store, 38, 1.0);
    assert!(player.render.appears_glowing);
    assert_eq!(
        player.render.outline_color, 0xffff_ffff,
        "players glow white"
    );
    assert_eq!(
        chicken.render.outline_color, 0,
        "non-players stay unoutlined"
    );

    store.set_spectator_outlines_key_down(false);
    let player = render_source(&store, 38, 1.0);
    assert_eq!(
        player.render.outline_color, 0,
        "releasing the key clears the glow"
    );
    assert!(!player.render.appears_glowing);

    store.set_spectator_outlines_key_down(true);
    store.apply_game_event(bbb_protocol::packets::GameEvent {
        event_id: 3,
        param: 0.0,
    });
    let player = render_source(&store, 38, 1.0);
    assert_eq!(
        player.render.outline_color, 0,
        "a survival viewer never spectator-glows"
    );
    assert!(!player.render.appears_glowing);
}

#[test]
fn entity_model_sources_project_effective_equipment_asset_ids_and_exact_trim_names() {
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
    configure_equipment_assets(
        &mut store,
        7,
        &["minecraft:diamond", "example:custom", "minecraft:saddle"],
        &[
            (DEFAULT_HEAD_ITEM, EquipmentSlot::Head, "minecraft:diamond"),
            (SADDLE_ITEM, EquipmentSlot::Saddle, "minecraft:saddle"),
            (BODY_ITEM, EquipmentSlot::Body, "example:custom"),
        ],
    );

    for (id, entity_type) in [
        (54, VANILLA_ENTITY_TYPE_ZOMBIE_ID),
        (55, VANILLA_ENTITY_TYPE_PIG_ID),
        (56, VANILLA_ENTITY_TYPE_WOLF_ID),
    ] {
        store.apply_add_entity(protocol_add_entity_with_type(id, entity_type));
    }

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
    let equip_type_id =
        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID;
    equip_slots(
        &mut store,
        54,
        [
            (
                EquipmentSlot::Head,
                ItemStackSummary {
                    item_id: Some(PATCHED_ITEM),
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added_type_ids: vec![equip_type_id],
                        equippable: Some(equippable(4, Some("example:custom"))),
                        enchantment_glint_override: Some(true),
                        dyed_color: Some(0x12_34_56),
                        ..direct_trim
                    }
                    .into(),
                },
            ),
            (
                EquipmentSlot::Chest,
                ItemStackSummary {
                    item_id: Some(PATCHED_ITEM),
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added_type_ids: vec![equip_type_id],
                        // Wings/SimpleEquipmentLayer do not require this
                        // component slot to match their source slot.
                        equippable: Some(equippable(4, Some("example:custom"))),
                        ..Default::default()
                    }
                    .into(),
                },
            ),
        ],
    );
    equip_slot(
        &mut store,
        55,
        EquipmentSlot::Saddle,
        ItemStackSummary {
            item_id: Some(SADDLE_ITEM),
            count: 1,
            component_patch: DataComponentPatchSummary {
                dyed_color: Some(SADDLE_DYE),
                ..Default::default()
            }
            .into(),
        },
    );
    equip_slot(
        &mut store,
        56,
        EquipmentSlot::Body,
        ItemStackSummary {
            item_id: Some(BODY_ITEM),
            count: 1,
            component_patch: DataComponentPatchSummary {
                dyed_color: Some(BODY_DYE),
                ..Default::default()
            }
            .into(),
        },
    );

    let zombie = render_source(&store, 54, 1.0);
    assert_eq!(
        zombie.render.head_equipment_asset,
        Some(EntityEquipmentAssetId::new(7, 1))
    );
    assert_eq!(
        zombie.render.chest_wings_equipment_asset,
        Some(EntityEquipmentAssetId::new(7, 1))
    );
    assert_eq!(zombie.render.head_armor_dye, Some(0x12_34_56));
    assert!(zombie.render.head_armor_foil);
    assert_eq!(
        zombie.armor_trims.head_armor_trim,
        Some(ArmorTrimSprite {
            sprite_id: "example:trims/entity/humanoid/spiral_ruby_custom".to_string(),
            decal: true,
        })
    );
    let pig = render_source(&store, 55, 1.0);
    assert_eq!(
        pig.render.kind.pig().saddle_equipment_asset,
        Some(EntityEquipmentAssetId::new(7, 2))
    );
    assert_eq!(pig.render.kind.pig().saddle_dye, Some(SADDLE_DYE as u32));
    let wolf = render_source(&store, 56, 1.0);
    assert_eq!(
        wolf.render.kind.wolf().body_equipment_asset,
        Some(EntityEquipmentAssetId::new(7, 1))
    );
    assert_eq!(
        wolf.render.kind.wolf().body_armor_dye,
        Some(BODY_DYE as u32)
    );

    equip_slot(
        &mut store,
        54,
        EquipmentSlot::Head,
        ItemStackSummary {
            item_id: Some(DEFAULT_HEAD_ITEM),
            count: 1,
            component_patch: DataComponentPatchSummary {
                removed_type_ids: vec![equip_type_id],
                enchantment_glint_override: Some(true),
                ..Default::default()
            }
            .into(),
        },
    );
    let removed = render_source(&store, 54, 1.0);
    assert_eq!(removed.render.head_equipment_asset, None);
    assert_eq!(removed.armor_trims.head_armor_trim, None);
    assert!(!removed.render.head_armor_foil);

    equip_slot(
        &mut store,
        54,
        EquipmentSlot::Head,
        plain_stack(DEFAULT_HEAD_ITEM, 1),
    );
    assert_eq!(
        render_source(&store, 54, 1.0).render.head_equipment_asset,
        Some(EntityEquipmentAssetId::new(7, 0))
    );

    equip_slot(
        &mut store,
        54,
        EquipmentSlot::Head,
        ItemStackSummary {
            item_id: Some(PATCHED_ITEM),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![equip_type_id],
                equippable: Some(equippable(3, Some("example:custom"))),
                ..Default::default()
            }
            .into(),
        },
    );
    assert_eq!(
        render_source(&store, 54, 1.0).render.head_equipment_asset,
        None,
        "humanoid armor requires the effective Equippable slot to match"
    );

    set_entity_data(
        &mut store,
        56,
        [baby_data(VANILLA_ENTITY_TYPE_WOLF_ID, true)],
    );
    let baby_wolf = render_source(&store, 56, 1.0);
    assert_eq!(
        baby_wolf.render.kind.wolf().body_equipment_asset,
        None,
        "the existing adult-only wolf armor model gate remains authoritative"
    );
    assert_eq!(
        baby_wolf.render.kind.wolf().body_armor_dye,
        None,
        "the adult-only equipment identity gate also suppresses its dye"
    );
}

#[test]
fn entity_model_sources_project_named_and_direct_armor_trim_sprites() {
    let iron_helmet = 804;
    let diamond_leggings = 805;
    let mut store = WorldStore::new();
    configure_equipment_assets(
        &mut store,
        8,
        &["minecraft:iron", "minecraft:diamond"],
        &[
            (iron_helmet, EquipmentSlot::Head, "minecraft:iron"),
            (diamond_leggings, EquipmentSlot::Legs, "minecraft:diamond"),
        ],
    );
    store.apply_add_entity(protocol_add_entity_with_type(
        52,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    ));

    let named = ItemStackSummary {
        item_id: Some(iron_helmet),
        count: 1,
        component_patch: DataComponentPatchSummary {
            // `minecraft:trim_material` and `minecraft:trim_pattern` are loaded
            // from data files (RegistryDataLoader.java:99-100), so a holder id
            // with no synchronized registry is a sorted-resource-id index.
            armor_trim_material_id: Some(5), // iron in loader order
            armor_trim_pattern_id: Some(8),  // sentry in loader order
            ..Default::default()
        }
        .into(),
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
        }
        .into(),
    };
    equip_slots(
        &mut store,
        52,
        [(EquipmentSlot::Head, named), (EquipmentSlot::Legs, direct)],
    );

    let source = render_source(&store, 52, 1.0);
    assert_eq!(
        source.armor_trims.head_armor_trim,
        trim_sprite("minecraft:trims/entity/humanoid/sentry_iron_darker")
    );
    assert_eq!(
        source.armor_trims.legs_armor_trim,
        Some(ArmorTrimSprite {
            sprite_id: "example:trims/entity/humanoid_leggings/spiral_ruby_darker".to_string(),
            decal: true,
        })
    );
    assert_eq!(source.armor_trims.chest_armor_trim, None);
    assert_eq!(source.armor_trims.feet_armor_trim, None);
}

#[test]
fn entity_model_sources_project_wings_trim_and_foil_from_chest_asset() {
    const ELYTRA_ITEM_ID: i32 = 806;
    const CHESTPLATE_ITEM_ID: i32 = 807;

    let wings = |store: &WorldStore| {
        let source = render_source(store, 53, 1.0);
        (
            source.armor_trims.chest_wings_trim,
            source.render.chest_wings_foil,
        )
    };

    let mut store = WorldStore::new();
    configure_equipment_assets(
        &mut store,
        9,
        &["minecraft:elytra"],
        &[(ELYTRA_ITEM_ID, EquipmentSlot::Chest, "minecraft:elytra")],
    );
    store.apply_add_entity(protocol_add_entity_with_type(
        53,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    ));

    equip_slot(
        &mut store,
        53,
        EquipmentSlot::Chest,
        sentry_iron_trimmed_stack(ELYTRA_ITEM_ID, 1, None),
    );
    assert_eq!(
        wings(&store),
        (
            trim_sprite("minecraft:trims/entity/wings/sentry_iron"),
            true
        )
    );

    equip_slot(
        &mut store,
        53,
        EquipmentSlot::Chest,
        sentry_iron_trimmed_stack(CHESTPLATE_ITEM_ID, 1, None),
    );
    assert_eq!(
        wings(&store),
        (None, false),
        "a chest item without a WINGS equipment asset cannot project wings trim or foil"
    );
}

#[test]
fn entity_model_sources_project_synchronized_custom_armor_trim_payloads() {
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
    store.begin_registry_configuration();
    crate::registries::tests::record_minimal_non_empty_synchronized_registries(&mut store, None);
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
    store.finish_registry_configuration().unwrap();
    configure_equipment_assets(
        &mut store,
        10,
        &["minecraft:iron"],
        &[(iron_helmet, EquipmentSlot::Head, "minecraft:iron")],
    );

    store.apply_add_entity(protocol_add_entity_with_type(
        53,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    ));
    let trim_helmet = |material_id: i32| ItemStackSummary {
        item_id: Some(iron_helmet),
        count: 1,
        component_patch: DataComponentPatchSummary {
            armor_trim_material_id: Some(material_id),
            armor_trim_pattern_id: Some(0),
            ..Default::default()
        }
        .into(),
    };
    equip_slot(&mut store, 53, EquipmentSlot::Head, trim_helmet(0));
    assert_eq!(
        render_source(&store, 53, 1.0).armor_trims.head_armor_trim,
        Some(ArmorTrimSprite {
            sprite_id: "example:trims/entity/humanoid/spire_opal_darker".to_string(),
            decal: true,
        })
    );

    equip_slot(&mut store, 53, EquipmentSlot::Head, trim_helmet(1));
    assert_eq!(
        render_source(&store, 53, 1.0).armor_trims.head_armor_trim,
        None,
        "a synchronized registry owns holder order; an out-of-range id must not use builtin order"
    );
}

#[test]
fn entity_model_sources_project_worn_armor_dye_colors() {
    let leather_chestplate = 810;
    let leather_boots = 811;
    let iron_helmet = 812;
    let dye = 0x3F_6CDA;

    let mut store = WorldStore::new();
    configure_equipment_assets(
        &mut store,
        11,
        &["minecraft:leather", "minecraft:iron"],
        &[
            (
                leather_chestplate,
                EquipmentSlot::Chest,
                "minecraft:leather",
            ),
            (leather_boots, EquipmentSlot::Feet, "minecraft:leather"),
            (iron_helmet, EquipmentSlot::Head, "minecraft:iron"),
        ],
    );
    store.apply_add_entity(protocol_add_entity_with_type(
        51,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    ));

    let dyed_armor_item = |item_id: i32, dyed_color: Option<i32>| ItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        component_patch: DataComponentPatchSummary {
            dyed_color,
            ..Default::default()
        }
        .into(),
    };
    equip_slots(
        &mut store,
        51,
        [
            (
                EquipmentSlot::Chest,
                dyed_armor_item(leather_chestplate, Some(dye)),
            ),
            (EquipmentSlot::Feet, dyed_armor_item(leather_boots, None)),
            (EquipmentSlot::Head, dyed_armor_item(iron_helmet, None)),
        ],
    );

    let source = render_source(&store, 51, 1.0);
    // Dye remains an independent presentation input carried beside the canonical asset id.
    assert_eq!(source.render.chest_armor_dye, Some(dye as u32));
    assert_eq!(source.render.feet_armor_dye, None);
    assert_eq!(source.render.head_armor_dye, None);
    assert_eq!(source.render.legs_armor_dye, None);
}

#[test]
fn entity_model_sources_project_snow_golem_pumpkin_flag() {
    let (pumpkin_data_id, pumpkin_default, _pumpkin_mask) =
        vanilla_entity_metadata_snow_golem_pumpkin_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID,
        )
        .expect("generated SnowGolem pumpkin accessor must resolve");
    let pumpkin = |store: &WorldStore, id: i32| {
        render_source(store, id, 0.0)
            .render
            .kind
            .snow_golem()
            .pumpkin
    };

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

    set_entity_data(&mut store, 70, [protocol_byte_data(pumpkin_data_id, 0)]);
    assert!(!pumpkin(&store, 70));

    set_entity_data(
        &mut store,
        70,
        [protocol_byte_data(pumpkin_data_id, pumpkin_default)],
    );
    assert!(pumpkin(&store, 70));
}

#[test]
fn entity_model_sources_project_mount_saddles_and_ridden_state() {
    const SADDLE_ITEM_ID: i32 = 820;
    const PLAIN_ITEM_ID: i32 = 821;

    #[derive(Clone, Copy)]
    enum SaddleMountFamily {
        Pig,
        Equine,
        Strider,
        Camel,
        Nautilus,
    }

    fn read_saddle_state(
        store: &WorldStore,
        entity_id: i32,
        family: SaddleMountFamily,
    ) -> (bool, Option<bool>, Option<ArmorTrimSprite>, bool) {
        let source = render_source(store, entity_id, 0.0);
        match family {
            SaddleMountFamily::Pig => (
                source.render.kind.pig().saddle_equipment_asset.is_some(),
                None,
                source.armor_trims.pig_saddle_trim,
                source.render.kind.pig().saddle_foil,
            ),
            SaddleMountFamily::Equine => (
                source.render.kind.equine().saddle_equipment_asset.is_some(),
                Some(source.render.kind.equine().saddle_ridden),
                source.armor_trims.equine_saddle_trim,
                source.render.kind.equine().saddle_foil,
            ),
            SaddleMountFamily::Strider => (
                source
                    .render
                    .kind
                    .strider()
                    .saddle_equipment_asset
                    .is_some(),
                Some(source.render.kind.strider().ridden),
                source.armor_trims.strider_saddle_trim,
                source.render.kind.strider().saddle_foil,
            ),
            SaddleMountFamily::Camel => (
                source.render.kind.camel().saddle_equipment_asset.is_some(),
                Some(source.render.kind.camel().saddle_ridden),
                source.armor_trims.camel_saddle_trim,
                source.render.kind.camel().saddle_foil,
            ),
            SaddleMountFamily::Nautilus => (
                source
                    .render
                    .kind
                    .nautilus()
                    .saddle_equipment_asset
                    .is_some(),
                None,
                source.armor_trims.nautilus_saddle_trim,
                source.render.kind.nautilus().saddle_foil,
            ),
        }
    }

    struct SaddleCase {
        family: SaddleMountFamily,
        primary_type: i32,
        primary_layer: &'static str,
        extra_variants: &'static [(i32, &'static str)],
        unrelated_types: &'static [i32],
        tracks_ridden: bool,
    }

    let cases = [
        SaddleCase {
            family: SaddleMountFamily::Pig,
            primary_type: VANILLA_ENTITY_TYPE_PIG_ID,
            primary_layer: "pig_saddle",
            extra_variants: &[],
            unrelated_types: &[VANILLA_ENTITY_TYPE_COW_ID],
            tracks_ridden: false,
        },
        SaddleCase {
            family: SaddleMountFamily::Equine,
            primary_type: VANILLA_ENTITY_TYPE_HORSE_ID,
            primary_layer: "horse_saddle",
            extra_variants: &[
                (VANILLA_ENTITY_TYPE_DONKEY_ID, "donkey_saddle"),
                (VANILLA_ENTITY_TYPE_MULE_ID, "mule_saddle"),
                (
                    VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
                    "skeleton_horse_saddle",
                ),
                (VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID, "zombie_horse_saddle"),
            ],
            unrelated_types: &[VANILLA_ENTITY_TYPE_PIG_ID, VANILLA_ENTITY_TYPE_COW_ID],
            tracks_ridden: true,
        },
        SaddleCase {
            family: SaddleMountFamily::Strider,
            primary_type: VANILLA_ENTITY_TYPE_STRIDER_ID,
            primary_layer: "strider_saddle",
            extra_variants: &[],
            unrelated_types: &[VANILLA_ENTITY_TYPE_COW_ID],
            tracks_ridden: true,
        },
        SaddleCase {
            family: SaddleMountFamily::Camel,
            primary_type: VANILLA_ENTITY_TYPE_CAMEL_ID,
            primary_layer: "camel_saddle",
            extra_variants: &[(VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, "camel_husk_saddle")],
            unrelated_types: &[VANILLA_ENTITY_TYPE_COW_ID],
            tracks_ridden: true,
        },
        SaddleCase {
            family: SaddleMountFamily::Nautilus,
            primary_type: VANILLA_ENTITY_TYPE_NAUTILUS_ID,
            primary_layer: "nautilus_saddle",
            extra_variants: &[(VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID, "nautilus_saddle")],
            unrelated_types: &[VANILLA_ENTITY_TYPE_COW_ID],
            tracks_ridden: false,
        },
    ];

    for (idx, case) in cases.iter().enumerate() {
        let mut store = WorldStore::new();
        configure_equipment_assets(
            &mut store,
            12 + idx as u64,
            &["minecraft:saddle"],
            &[(SADDLE_ITEM_ID, EquipmentSlot::Saddle, "minecraft:saddle")],
        );
        let primary_id = 60;
        let passenger_id = 99;
        store.apply_add_entity(protocol_add_entity_with_type(primary_id, case.primary_type));
        store.apply_add_entity(protocol_add_entity_with_type(
            passenger_id,
            VANILLA_ENTITY_TYPE_COW_ID,
        ));

        let unequipped_ridden = case.tracks_ridden.then_some(false);
        assert_eq!(
            read_saddle_state(&store, primary_id, case.family),
            (false, unequipped_ridden, None, false)
        );

        if case.tracks_ridden {
            set_passengers(&mut store, primary_id, &[passenger_id]);
            assert_eq!(
                read_saddle_state(&store, primary_id, case.family),
                (false, Some(true), None, false),
                "ridden state is independent of whether the saddle slot resolves"
            );
            set_passengers(&mut store, primary_id, &[]);
        }

        equip_slot(
            &mut store,
            primary_id,
            EquipmentSlot::Saddle,
            sentry_iron_trimmed_stack(SADDLE_ITEM_ID, 1, None),
        );
        let expected_primary_trim = trim_sprite(&format!(
            "minecraft:trims/entity/{}/sentry_iron",
            case.primary_layer
        ));
        assert_eq!(
            read_saddle_state(&store, primary_id, case.family),
            (true, unequipped_ridden, expected_primary_trim.clone(), true)
        );

        if case.tracks_ridden {
            set_passengers(&mut store, primary_id, &[passenger_id]);
            assert_eq!(
                read_saddle_state(&store, primary_id, case.family),
                (true, Some(true), expected_primary_trim, true)
            );
        }

        for (offset, &(variant_type, layer)) in case.extra_variants.iter().enumerate() {
            let entity_id = 70 + offset as i32;
            store.apply_add_entity(protocol_add_entity_with_type(entity_id, variant_type));
            equip_slot(
                &mut store,
                entity_id,
                EquipmentSlot::Saddle,
                sentry_iron_trimmed_stack(SADDLE_ITEM_ID, 1, None),
            );
            assert_eq!(
                read_saddle_state(&store, entity_id, case.family),
                (
                    true,
                    unequipped_ridden,
                    trim_sprite(&format!("minecraft:trims/entity/{layer}/sentry_iron")),
                    true,
                )
            );
        }

        for (offset, &unrelated_type) in case.unrelated_types.iter().enumerate() {
            let entity_id = 80 + offset as i32;
            store.apply_add_entity(protocol_add_entity_with_type(entity_id, unrelated_type));
            equip_slot(
                &mut store,
                entity_id,
                EquipmentSlot::Saddle,
                plain_stack(SADDLE_ITEM_ID, 1),
            );
            assert_eq!(
                read_saddle_state(&store, entity_id, case.family),
                (false, unequipped_ridden, None, false)
            );
        }

        let ridden_after = case.tracks_ridden.then_some(true);
        equip_slot(
            &mut store,
            primary_id,
            EquipmentSlot::Saddle,
            plain_stack(PLAIN_ITEM_ID, 1),
        );
        assert_eq!(
            read_saddle_state(&store, primary_id, case.family),
            (false, ridden_after, None, false)
        );
        equip_slot(
            &mut store,
            primary_id,
            EquipmentSlot::Saddle,
            ItemStackSummary::empty(),
        );
        assert_eq!(
            read_saddle_state(&store, primary_id, case.family),
            (false, ridden_after, None, false)
        );
    }
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

    let animate_tail = |store: &WorldStore, id: i32| {
        render_source(store, id, 0.0)
            .render
            .kind
            .equine()
            .animate_tail
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
        let source = render_source(store, 68, partial_tick);
        let equine = source.render.kind.equine();
        (
            equine.eat_animation,
            equine.stand_animation,
            equine.feeding_animation,
        )
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        68,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    assert_eq!(pose(&store, 1.0), (0.0, 0.0, 0.0));

    set_entity_data(
        &mut store,
        68,
        [protocol_byte_data(
            accessors.flags_data_id,
            accessors.eating_mask | accessors.open_mouth_mask,
        )],
    );
    assert_eq!(pose(&store, 1.0), (0.0, 0.0, 0.0));
    store.advance_entity_client_animations(1);
    assert_close(pose(&store, 0.5).0, 0.225, "eat partial");
    assert_close(pose(&store, 1.0).0, 0.45, "eat rise");
    assert_eq!(pose(&store, 1.0).1, 0.0);
    assert_close(pose(&store, 1.0).2, 0.75, "mouth rise");

    set_entity_data(
        &mut store,
        68,
        [protocol_byte_data(
            accessors.flags_data_id,
            accessors.standing_mask | accessors.open_mouth_mask,
        )],
    );
    store.advance_entity_client_animations(1);
    assert_eq!(
        pose(&store, 1.0).0,
        0.0,
        "vanilla clears eatAnim while standing"
    );
    assert_close(pose(&store, 1.0).1, 0.45, "stand rise");
    assert_close(pose(&store, 1.0).2, 0.975, "mouth second rise");

    set_entity_data(
        &mut store,
        68,
        [protocol_byte_data(accessors.flags_data_id, 0)],
    );
    store.advance_entity_client_animations(1);
    assert_close(pose(&store, 1.0).1, 0.17374, "stand cubic falloff");
    assert_close(pose(&store, 1.0).2, 0.2425, "mouth falloff");
}

#[test]
fn entity_model_sources_project_nautilus_horse_and_llama_body_equipment_from_body_slot() {
    // Nautilus body armor + `body_item_present`
    {
        const IRON_ARMOR: i32 = 841;
        const NETHERITE_ARMOR: i32 = 842;
        const PLAIN_ITEM: i32 = 843;
        let state = |store: &WorldStore, id: i32| {
            let source = render_source(store, id, 0.0);
            (
                source.render.kind.nautilus().body_equipment_asset,
                source.armor_trims.nautilus_body_armor_trim,
                source.render.kind.nautilus().body_armor_foil,
                source.render.kind.nautilus().body_item_present,
            )
        };

        let mut store = WorldStore::new();
        for (id, entity_type) in [
            (78, VANILLA_ENTITY_TYPE_NAUTILUS_ID),
            (79, VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID),
            (80, VANILLA_ENTITY_TYPE_COW_ID),
            (81, VANILLA_ENTITY_TYPE_NAUTILUS_ID),
        ] {
            store.apply_add_entity(protocol_add_entity_with_type(id, entity_type));
        }
        set_entity_data(
            &mut store,
            81,
            [baby_data(VANILLA_ENTITY_TYPE_NAUTILUS_ID, true)],
        );
        for (id, item_id) in [
            (78, IRON_ARMOR),
            (79, NETHERITE_ARMOR),
            (80, IRON_ARMOR),
            (81, IRON_ARMOR),
        ] {
            equip_slot(
                &mut store,
                id,
                EquipmentSlot::Body,
                sentry_iron_trimmed_stack(item_id, 1, None),
            );
        }
        assert_eq!(state(&store, 78), (None, None, false, true));
        assert!(state(&store, 79).3);
        assert!(!state(&store, 80).3);
        assert!(state(&store, 81).3);

        configure_equipment_assets(
            &mut store,
            20,
            &["minecraft:iron", "minecraft:netherite"],
            &[
                (IRON_ARMOR, EquipmentSlot::Body, "minecraft:iron"),
                (NETHERITE_ARMOR, EquipmentSlot::Body, "minecraft:netherite"),
            ],
        );
        assert_eq!(
            state(&store, 78),
            (
                Some(EntityEquipmentAssetId::new(20, 0)),
                trim_sprite("minecraft:trims/entity/nautilus_body/sentry_iron_darker"),
                true,
                true,
            )
        );
        assert_eq!(
            state(&store, 79),
            (
                Some(EntityEquipmentAssetId::new(20, 1)),
                trim_sprite("minecraft:trims/entity/nautilus_body/sentry_iron"),
                true,
                true,
            )
        );
        assert_eq!(state(&store, 80), (None, None, false, false));
        assert_eq!(state(&store, 81), (None, None, false, true));

        equip_slot(
            &mut store,
            78,
            EquipmentSlot::Body,
            sentry_iron_trimmed_stack(PLAIN_ITEM, 1, None),
        );
        assert_eq!(state(&store, 78), (None, None, false, true));
        equip_slot(
            &mut store,
            78,
            EquipmentSlot::Body,
            sentry_iron_trimmed_stack(PLAIN_ITEM, 0, None),
        );
        assert!(!state(&store, 78).3);
    }

    // Horse body armor + dye + skeleton horse / baby exclusion
    {
        const LEATHER_ARMOR: i32 = 844;
        const DIAMOND_ARMOR: i32 = 845;
        const NETHERITE_ARMOR: i32 = 846;
        const PLAIN_ITEM: i32 = 847;
        const LEATHER_DYE: i32 = 0x0033_66CC;
        let state = |store: &WorldStore, id: i32| {
            let source = render_source(store, id, 0.0);
            (
                source.render.kind.equine().body_equipment_asset,
                source.render.kind.equine().body_armor_dye,
                source.armor_trims.equine_body_armor_trim,
                source.render.kind.equine().body_armor_foil,
            )
        };

        let mut store = WorldStore::new();
        for (id, entity_type) in [
            (82, VANILLA_ENTITY_TYPE_HORSE_ID),
            (83, VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID),
            (84, VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID),
            (85, VANILLA_ENTITY_TYPE_COW_ID),
            (86, VANILLA_ENTITY_TYPE_HORSE_ID),
        ] {
            store.apply_add_entity(protocol_add_entity_with_type(id, entity_type));
        }
        set_entity_data(
            &mut store,
            86,
            [baby_data(VANILLA_ENTITY_TYPE_HORSE_ID, true)],
        );
        for (id, item_id, dye) in [
            (82, LEATHER_ARMOR, Some(LEATHER_DYE)),
            (83, NETHERITE_ARMOR, None),
            (84, DIAMOND_ARMOR, None),
            (85, LEATHER_ARMOR, Some(LEATHER_DYE)),
            (86, LEATHER_ARMOR, Some(LEATHER_DYE)),
        ] {
            equip_slot(
                &mut store,
                id,
                EquipmentSlot::Body,
                sentry_iron_trimmed_stack(item_id, 1, dye),
            );
        }
        assert_eq!(state(&store, 82), (None, None, None, false));

        configure_equipment_assets(
            &mut store,
            21,
            &[
                "minecraft:leather",
                "minecraft:diamond",
                "minecraft:netherite",
            ],
            &[
                (LEATHER_ARMOR, EquipmentSlot::Body, "minecraft:leather"),
                (DIAMOND_ARMOR, EquipmentSlot::Body, "minecraft:diamond"),
                (NETHERITE_ARMOR, EquipmentSlot::Body, "minecraft:netherite"),
            ],
        );
        let horse_trim = trim_sprite("minecraft:trims/entity/horse_body/sentry_iron");
        assert_eq!(
            state(&store, 82),
            (
                Some(EntityEquipmentAssetId::new(21, 0)),
                Some(LEATHER_DYE as u32),
                horse_trim.clone(),
                true,
            )
        );
        assert_eq!(
            state(&store, 83),
            (
                Some(EntityEquipmentAssetId::new(21, 2)),
                None,
                horse_trim,
                true
            )
        );
        for excluded_id in [84, 85, 86] {
            assert_eq!(state(&store, excluded_id), (None, None, None, false));
        }

        equip_slot(
            &mut store,
            82,
            EquipmentSlot::Body,
            sentry_iron_trimmed_stack(PLAIN_ITEM, 1, Some(LEATHER_DYE)),
        );
        assert_eq!(state(&store, 82), (None, None, None, false));
    }

    // Llama & trader llama carpet decor
    {
        const WHITE_CARPET: i32 = 838;
        const BLACK_CARPET: i32 = 839;
        const PLAIN_ITEM: i32 = 840;
        let state = |store: &WorldStore, id: i32| {
            let source = render_source(store, id, 0.0);
            (
                source.render.kind.llama().body_equipment_asset,
                source.armor_trims.llama_body_decor_trim,
                source.render.kind.llama().body_decor_foil,
            )
        };

        let mut store = WorldStore::new();
        for (id, entity_type) in [
            (74, VANILLA_ENTITY_TYPE_LLAMA_ID),
            (75, VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID),
            (76, VANILLA_ENTITY_TYPE_COW_ID),
            (77, VANILLA_ENTITY_TYPE_LLAMA_ID),
        ] {
            store.apply_add_entity(protocol_add_entity_with_type(id, entity_type));
        }
        set_entity_data(
            &mut store,
            77,
            [baby_data(VANILLA_ENTITY_TYPE_LLAMA_ID, true)],
        );
        for (id, item_id) in [
            (74, WHITE_CARPET),
            (75, BLACK_CARPET),
            (76, WHITE_CARPET),
            (77, WHITE_CARPET),
        ] {
            equip_slot(
                &mut store,
                id,
                EquipmentSlot::Body,
                sentry_iron_trimmed_stack(item_id, 1, None),
            );
        }
        assert_eq!(state(&store, 74), (None, None, false));

        configure_equipment_assets(
            &mut store,
            23,
            &["minecraft:white_carpet", "minecraft:black_carpet"],
            &[
                (WHITE_CARPET, EquipmentSlot::Body, "minecraft:white_carpet"),
                (BLACK_CARPET, EquipmentSlot::Body, "minecraft:black_carpet"),
            ],
        );
        let llama_trim = trim_sprite("minecraft:trims/entity/llama_body/sentry_iron");
        assert_eq!(
            state(&store, 74),
            (
                Some(EntityEquipmentAssetId::new(23, 0)),
                llama_trim.clone(),
                true
            )
        );
        assert_eq!(
            state(&store, 75),
            (Some(EntityEquipmentAssetId::new(23, 1)), llama_trim, true)
        );
        assert_eq!(state(&store, 76), (None, None, false));
        assert_eq!(state(&store, 77), (None, None, false));

        equip_slot(
            &mut store,
            74,
            EquipmentSlot::Body,
            sentry_iron_trimmed_stack(PLAIN_ITEM, 1, None),
        );
        assert_eq!(state(&store, 74), (None, None, false));
        equip_slot(
            &mut store,
            75,
            EquipmentSlot::Body,
            sentry_iron_trimmed_stack(BLACK_CARPET, 0, None),
        );
        assert_eq!(state(&store, 75), (None, None, false));
    }
}

#[test]
fn entity_model_sources_project_wolf_body_armor_from_body_slot() {
    const WOLF_ARMOR_ITEM_ID: i32 = 848;
    const PLAIN_ITEM_ID: i32 = 849;
    const WOLF_ARMOR_DYE: i32 = 0x0033_66CC;

    fn wolf_stack(
        item_id: i32,
        count: i32,
        dyed_color: Option<i32>,
        damage: Option<i32>,
        unbreakable: bool,
        enchanted: bool,
        enchantment_glint_override: Option<bool>,
    ) -> ItemStackSummary {
        let mut added_type_ids = Vec::new();
        if damage.is_some() {
            added_type_ids
                .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID);
        }
        if unbreakable {
            added_type_ids.push(
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID,
            );
        }
        let enchantments = if enchanted {
            vec![ItemEnchantmentSummary {
                holder_id: 12,
                level: 1,
            }]
        } else {
            Vec::new()
        };
        ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: DataComponentPatchSummary {
                added_type_ids,
                dyed_color,
                damage,
                unbreakable,
                enchantments,
                enchantment_glint_override,
                armor_trim_material_id: Some(5), // iron in loader order
                armor_trim_pattern_id: Some(8),  // sentry in loader order
                ..Default::default()
            }
            .into(),
        }
    }
    let wolf_body_armor = |store: &WorldStore, entity_id: i32| {
        let source = render_source(store, entity_id, 0.0);
        (
            source.render.kind.wolf().body_equipment_asset,
            source.render.kind.wolf().body_armor_dye,
            source.render.kind.wolf().body_armor_crackiness,
            source.render.kind.wolf().body_armor_foil,
            source.armor_trims.wolf_body_armor_trim,
        )
    };

    let mut store = WorldStore::new();
    for (id, entity_type) in [
        (87, VANILLA_ENTITY_TYPE_WOLF_ID),
        (88, VANILLA_ENTITY_TYPE_COW_ID),
        (89, VANILLA_ENTITY_TYPE_WOLF_ID),
    ] {
        store.apply_add_entity(protocol_add_entity_with_type(id, entity_type));
    }
    set_entity_data(
        &mut store,
        89,
        [baby_data(VANILLA_ENTITY_TYPE_WOLF_ID, true)],
    );

    for entity_id in [87, 88, 89] {
        equip_slot(
            &mut store,
            entity_id,
            EquipmentSlot::Body,
            wolf_stack(
                WOLF_ARMOR_ITEM_ID,
                1,
                Some(WOLF_ARMOR_DYE),
                Some(4),
                false,
                true,
                None,
            ),
        );
    }
    assert_eq!(
        wolf_body_armor(&store, 87),
        (None, None, WolfArmorCrackiness::None, false, None)
    );

    store.items.commit_item_component_defaults(BTreeMap::from([(
        WOLF_ARMOR_ITEM_ID,
        DataComponentPatchSummary {
            added_type_ids: vec![
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
            ],
            max_stack_size: Some(1),
            max_damage: Some(64),
            damage: Some(0),
            ..Default::default()
        },
    )]));
    configure_equipment_assets(
        &mut store,
        22,
        &["minecraft:armadillo_scute"],
        &[(
            WOLF_ARMOR_ITEM_ID,
            EquipmentSlot::Body,
            "minecraft:armadillo_scute",
        )],
    );
    assert_eq!(
        wolf_body_armor(&store, 87),
        (
            Some(EntityEquipmentAssetId::new(22, 0)),
            Some(WOLF_ARMOR_DYE as u32),
            WolfArmorCrackiness::Low,
            true,
            trim_sprite("minecraft:trims/entity/wolf_body/sentry_iron"),
        )
    );
    for id in [88, 89] {
        assert_eq!(
            wolf_body_armor(&store, id),
            (None, None, WolfArmorCrackiness::None, false, None)
        );
    }

    for (damage, expected) in [
        (24, WolfArmorCrackiness::Medium),
        (44, WolfArmorCrackiness::High),
    ] {
        equip_slot(
            &mut store,
            87,
            EquipmentSlot::Body,
            wolf_stack(
                WOLF_ARMOR_ITEM_ID,
                1,
                None,
                Some(damage),
                false,
                false,
                None,
            ),
        );
        assert_eq!(wolf_body_armor(&store, 87).2, expected);
    }

    equip_slot(
        &mut store,
        87,
        EquipmentSlot::Body,
        wolf_stack(
            WOLF_ARMOR_ITEM_ID,
            1,
            None,
            Some(44),
            true,
            true,
            Some(false),
        ),
    );
    assert_eq!(wolf_body_armor(&store, 87).2, WolfArmorCrackiness::None);
    assert!(!wolf_body_armor(&store, 87).3);

    equip_slot(
        &mut store,
        87,
        EquipmentSlot::Body,
        wolf_stack(
            PLAIN_ITEM_ID,
            0,
            Some(WOLF_ARMOR_DYE),
            Some(44),
            false,
            false,
            None,
        ),
    );
    assert_eq!(
        wolf_body_armor(&store, 87),
        (None, None, WolfArmorCrackiness::None, false, None)
    );
}

#[test]
fn entity_model_sources_project_happy_ghast_harness_leash_holder_and_ridden_state() {
    const WHITE_HARNESS_ITEM_ID: i32 = 841;
    const PLAIN_ITEM_ID: i32 = 842;
    let state = |store: &WorldStore, entity_id: i32| {
        let source = render_source(store, entity_id, 0.0);
        let ghast = source.render.kind.happy_ghast();
        (
            ghast.body_item,
            ghast.body_equipment_asset,
            source.armor_trims.happy_ghast_harness_trim,
            ghast.harness_foil,
            ghast.body_item_is_harness,
            ghast.is_leash_holder,
            ghast.ridden,
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
    for (id, entity_type) in [
        (88, VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID),
        (89, VANILLA_ENTITY_TYPE_COW_ID),
        (90, VANILLA_ENTITY_TYPE_PLAYER_ID),
    ] {
        store.apply_add_entity(protocol_add_entity_with_type(id, entity_type));
    }
    for id in [88, 89] {
        equip_slot(
            &mut store,
            id,
            EquipmentSlot::Body,
            sentry_iron_trimmed_stack(PLAIN_ITEM_ID, 1, None),
        );
    }
    assert_eq!(
        state(&store, 88),
        (true, None, None, false, false, false, false)
    );
    assert_eq!(
        state(&store, 89),
        (false, None, None, false, false, false, false)
    );

    configure_equipment_assets(
        &mut store,
        24,
        &["minecraft:white_harness"],
        &[(
            WHITE_HARNESS_ITEM_ID,
            EquipmentSlot::Body,
            "minecraft:white_harness",
        )],
    );
    equip_slot(
        &mut store,
        88,
        EquipmentSlot::Body,
        sentry_iron_trimmed_stack(WHITE_HARNESS_ITEM_ID, 1, None),
    );
    assert_eq!(
        state(&store, 88),
        (
            true,
            Some(EntityEquipmentAssetId::new(24, 0)),
            trim_sprite("minecraft:trims/entity/happy_ghast_body/sentry_iron"),
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
    set_entity_data(
        &mut store,
        88,
        [protocol_bool_data(accessors.is_leash_holder_data_id, true)],
    );
    assert!(state(&store, 88).5);
    set_passengers(&mut store, 88, &[90]);
    assert!(state(&store, 88).6);

    equip_slot(
        &mut store,
        88,
        EquipmentSlot::Body,
        sentry_iron_trimmed_stack(WHITE_HARNESS_ITEM_ID, 0, None),
    );
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

    let mut store = WorldStore::with_dimension(crate::WorldDimension {
        min_y: 0,
        height: 16,
    });
    store.insert_decoded_chunk(empty_test_chunk());
    for (id, entity_type_id, z) in [
        (50, VANILLA_ENTITY_TYPE_COD_ID, 8.5),
        (52, VANILLA_ENTITY_TYPE_HORSE_ID, 10.5),
    ] {
        store.apply_add_entity(ProtocolAddEntity {
            id,
            entity_type_id,
            position: ProtocolVec3d { x: 8.5, y: 2.0, z },
            ..protocol_add_entity_with_type(id, entity_type_id)
        });
        assert!(!render_source(&store, id, 1.0).render.in_water);
    }

    for z in [8, 10] {
        assert!(store.apply_block_update(ProtocolBlockUpdate {
            pos: ProtocolBlockPos { x: 8, y: 2, z },
            block_state_id: SOURCE_WATER_BLOCK_STATE_ID,
        }));
    }
    for id in [50, 52] {
        let wet = render_source(&store, id, 1.0);
        assert!(wet.render.in_water);
        assert!(!wet.render.kind.boat().underwater);
    }
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
        entity_type_id: VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        position: ProtocolVec3d {
            x: 8.5,
            y: 2.0,
            z: 8.5,
        },
        ..protocol_add_entity_with_type(51, VANILLA_ENTITY_TYPE_OAK_BOAT_ID)
    });

    let underwater =
        |store: &WorldStore| render_source(store, 51, 1.0).render.kind.boat().underwater;
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
