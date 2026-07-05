use super::*;

#[test]
fn transient_entity_event_ignored_counters_update_world_counters() {
    let (tx, mut rx) = mpsc::channel(3);
    tx.try_send(NetEvent::Play(PlayClientbound::EntityAnimation(
        EntityAnimation { id: 999, action: 4 },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 999,
        event_id: 21,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::HurtAnimation(
        HurtAnimation { id: 999, yaw: 90.0 },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        3
    );

    let world_counters = world.counters();
    assert_eq!(world_counters.entity_animation_updates_received, 1);
    assert_eq!(world_counters.entity_animation_updates_applied, 0);
    assert_eq!(world_counters.entity_animation_updates_ignored, 1);
    assert_eq!(world_counters.entity_events_received, 1);
    assert_eq!(world_counters.entity_events_applied, 0);
    assert_eq!(world_counters.entity_events_ignored, 1);
    assert_eq!(world_counters.entity_hurt_animations_received, 1);
    assert_eq!(world_counters.entity_hurt_animations_applied, 0);
    assert_eq!(world_counters.entity_hurt_animations_ignored, 1);
}

#[test]
fn simple_entity_update_ignored_counters_update_world_counters() {
    let (tx, mut rx) = mpsc::channel(3);
    tx.try_send(NetEvent::Play(PlayClientbound::SetEntityMotion(
        SetEntityMotion {
            id: 999,
            delta_movement: ProtocolVec3d::default(),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::RotateHead(RotateHead {
        id: 999,
        y_head_rot: 90.0,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetEntityLink(
        SetEntityLink {
            source_id: 999,
            dest_id: 123,
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        3
    );

    let world_counters = world.counters();
    assert_eq!(world_counters.entity_motion_updates_received, 1);
    assert_eq!(world_counters.entity_motion_updates_applied, 0);
    assert_eq!(world_counters.entity_motion_updates_ignored, 1);
    assert_eq!(world_counters.entity_head_rotations_received, 1);
    assert_eq!(world_counters.entity_head_rotations_applied, 0);
    assert_eq!(world_counters.entity_head_rotations_ignored, 1);
    assert_eq!(world_counters.entity_link_updates_received, 1);
    assert_eq!(world_counters.entity_link_updates_applied, 0);
    assert_eq!(world_counters.entity_link_updates_ignored, 1);
}

#[test]
fn entity_metadata_ignored_counters_update_world_counters() {
    let (tx, mut rx) = mpsc::channel(4);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(124, VANILLA_ENTITY_TYPE_ITEM_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetEquipment(
        SetEquipment {
            entity_id: 124,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Head,
                item: item_stack(42, 1),
            }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::UpdateAttributes(
        UpdateAttributes {
            entity_id: 124,
            attributes: vec![AttributeSnapshot {
                attribute_id: 21,
                base: 20.0,
                modifiers: Vec::new(),
            }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetEntityData(
        SetEntityData {
            id: 999,
            values: vec![EntityDataValue {
                data_id: 0,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(0x20),
            }],
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        4
    );

    let world_counters = world.counters();
    assert_eq!(world_counters.entity_data_updates_received, 1);
    assert_eq!(world_counters.entity_data_values_received, 1);
    assert_eq!(world_counters.entity_data_updates_applied, 0);
    assert_eq!(world_counters.entity_data_updates_ignored, 1);
    assert_eq!(world_counters.entity_equipment_updates_received, 1);
    assert_eq!(world_counters.entity_equipment_slots_received, 1);
    assert_eq!(world_counters.entity_equipment_updates_applied, 0);
    assert_eq!(world_counters.entity_equipment_updates_ignored, 1);
    assert_eq!(world_counters.entity_attribute_updates_received, 1);
    assert_eq!(world_counters.entity_attributes_received, 1);
    assert_eq!(world_counters.entity_attribute_updates_applied, 0);
    assert_eq!(world_counters.entity_attribute_updates_ignored, 1);
}

#[test]
fn passenger_ignored_counters_update_world_counters() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::SetPassengers(
        SetPassengers {
            vehicle_id: 999,
            passenger_ids: vec![123, 124],
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );

    let world_counters = world.counters();
    assert_eq!(world_counters.entity_passenger_updates_received, 1);
    assert_eq!(world_counters.entity_passenger_ids_received, 2);
    assert_eq!(world_counters.entity_passenger_updates_applied, 0);
    assert_eq!(world_counters.entity_passenger_updates_ignored, 1);
}

#[test]
fn remove_entities_updates_world_active_effect_counters() {
    let entity_id = 55;
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::RemoveEntities(
        RemoveEntities {
            entity_ids: vec![entity_id],
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(entity_id));
    world.apply_update_mob_effect(protocol_update_mob_effect(entity_id, 3));
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );

    assert_eq!(world.counters().entities_tracked, 0);
    assert_eq!(world.counters().active_mob_effects_tracked, 0);
}

#[test]
fn add_entity_replacement_updates_world_active_effect_counters() {
    let entity_id = 55;
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity(entity_id),
    )))
    .unwrap();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(entity_id));
    world.apply_update_mob_effect(protocol_update_mob_effect(entity_id, 3));
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );

    assert_eq!(world.counters().entities_tracked, 1);
    assert_eq!(world.counters().active_mob_effects_tracked, 0);
}

#[test]
fn mob_effect_ignored_counters_update_world_counters() {
    let (tx, mut rx) = mpsc::channel(3);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(124, VANILLA_ENTITY_TYPE_ITEM_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::UpdateMobEffect(
        protocol_update_mob_effect(124, 3),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::RemoveMobEffect(
        bbb_protocol::packets::RemoveMobEffect {
            entity_id: 124,
            effect_id: 3,
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        3
    );

    let world_counters = world.counters();
    assert_eq!(world_counters.update_mob_effect_packets, 1);
    assert_eq!(world_counters.update_mob_effects_ignored, 1);
    assert_eq!(world_counters.remove_mob_effect_packets, 1);
    assert_eq!(world_counters.remove_mob_effects_ignored, 1);
    assert_eq!(world_counters.active_mob_effects_tracked, 0);
}

#[test]
fn merchant_offers_event_updates_world_inventory_state_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::OpenScreen(OpenScreen {
        container_id: 7,
        menu_type_id: 19,
        title: "Merchant".to_string(),
        title_styled: Vec::new(),
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::MerchantOffers(
        MerchantOffers {
            container_id: 7,
            offers: vec![MerchantOffer {
                buy_a: item_cost(42, 3),
                sell: item_stack(99, 1),
                buy_b: Some(item_cost(43, 2)),
                is_out_of_stock: true,
                uses: 4,
                max_uses: 12,
                xp: 8,
                special_price_diff: -2,
                price_multiplier: 0.05,
                demand: 6,
            }],
            villager_level: 3,
            villager_xp: 120,
            show_progress: true,
            can_restock: false,
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );

    let container = world.inventory().open_container.as_ref().unwrap();
    let offers = container.merchant_offers.as_ref().unwrap();
    assert_eq!(offers.container_id, 7);
    assert_eq!(offers.offers.len(), 1);
    assert_eq!(offers.offers[0].buy_a, item_cost(42, 3));
    assert_eq!(offers.offers[0].sell, item_stack(99, 1));
    assert_eq!(offers.villager_level, 3);
    assert_eq!(offers.villager_xp, 120);
    assert!(offers.show_progress);
    assert!(!offers.can_restock);

    let world_counters = world.counters();
    assert_eq!(world_counters.container_open_updates_received, 1);
    assert_eq!(world_counters.merchant_offer_packets_received, 1);
    assert_eq!(world_counters.merchant_offer_packets_applied, 1);
    assert_eq!(world_counters.merchant_offer_packets_ignored, 0);
    assert_eq!(world_counters.merchant_offers_tracked, 1);
}

#[test]
fn recipe_book_events_update_world_state_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(3);
    tx.try_send(NetEvent::Play(PlayClientbound::RecipeBookAdd(
        RecipeBookAdd {
            replace: true,
            entries: vec![
                recipe_book_entry(7, true, true),
                recipe_book_entry(8, false, false),
            ],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::RecipeBookRemove(
        RecipeBookRemove {
            recipe_ids: vec![RecipeDisplayId { index: 8 }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::RecipeBookSettings(
        RecipeBookSettings {
            crafting: RecipeBookTypeSettings {
                open: true,
                filtering: false,
            },
            furnace: RecipeBookTypeSettings {
                open: false,
                filtering: true,
            },
            blast_furnace: RecipeBookTypeSettings {
                open: true,
                filtering: true,
            },
            smoker: RecipeBookTypeSettings {
                open: false,
                filtering: false,
            },
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        3
    );
    assert!(world.recipe_book().known.contains_key(&7));
    assert!(!world.recipe_book().known.contains_key(&8));
    assert!(world.recipe_book().highlights.contains(&7));
    assert_eq!(world.recipe_book().notification_ids, vec![7]);
    assert!(world.recipe_book().settings.crafting.open);
    assert!(world.recipe_book().settings.furnace.filtering);

    let world_counters = world.counters();
    assert_eq!(world_counters.recipe_book_add_packets, 1);
    assert_eq!(world_counters.recipe_book_remove_packets, 1);
    assert_eq!(world_counters.recipe_book_settings_packets, 1);
    assert_eq!(world_counters.recipe_book_replace_packets, 1);
    assert_eq!(world_counters.recipe_book_entries_received, 2);
    assert_eq!(world_counters.recipe_book_removed_entries_received, 1);
    assert_eq!(world_counters.recipe_book_entries_tracked, 1);
    assert_eq!(world_counters.recipe_book_highlights_tracked, 1);
    assert_eq!(world_counters.recipe_book_notifications_received, 1);
}

#[test]
fn update_advancements_event_updates_world_state_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::UpdateAdvancements(
        UpdateAdvancements {
            reset: true,
            added: vec![AdvancementSummary {
                id: "minecraft:story/root".to_string(),
                parent: None,
                display: None,
                requirements: vec![vec!["mine_stone".to_string(), "get_log".to_string()]],
                sends_telemetry_event: true,
            }],
            removed: Vec::new(),
            progress: vec![AdvancementProgressSummary {
                id: "minecraft:story/root".to_string(),
                criteria: vec![AdvancementCriterionProgressSummary {
                    name: "mine_stone".to_string(),
                    obtained_epoch_millis: Some(1_700_000_000_000),
                }],
            }],
            show_advancements: true,
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );

    assert!(world
        .advancements()
        .advancements
        .contains_key("minecraft:story/root"));
    let progress = world
        .advancements()
        .progress
        .get("minecraft:story/root")
        .unwrap();
    assert_eq!(progress.criteria.len(), 2);

    let world_counters = world.counters();
    assert_eq!(world_counters.update_advancements_packets, 1);
    assert_eq!(world_counters.update_advancements_reset_packets, 1);
    assert_eq!(world_counters.update_advancements_show_packets, 1);
    assert_eq!(world_counters.advancements_added_received, 1);
    assert_eq!(world_counters.advancements_removed_received, 0);
    assert_eq!(world_counters.advancements_adds_ignored, 0);
    assert_eq!(world_counters.advancement_progress_received, 1);
    assert_eq!(world_counters.advancement_progress_updates_ignored, 0);
    assert_eq!(world_counters.advancements_tracked, 1);
    assert_eq!(world_counters.advancement_roots_tracked, 1);
    assert_eq!(world_counters.advancement_progress_tracked, 1);
    assert_eq!(world_counters.advancement_progress_criteria_tracked, 2);
}

#[test]
fn update_recipes_event_replaces_world_recipe_access_state_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::UpdateRecipes(
        UpdateRecipes {
            property_sets: vec![RecipePropertySetSummary {
                key: "minecraft:furnace_input".to_string(),
                item_ids: vec![42, 43],
            }],
            stonecutter_recipes: vec![StonecutterSelectableRecipeSummary {
                input: IngredientSummary {
                    tag: None,
                    item_ids: vec![11, 12],
                },
                option_display: SlotDisplaySummary {
                    display_type_id: 4,
                    raw_payload: vec![4, 77],
                    item_stack: None,
                },
            }],
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );

    assert_eq!(
        world.recipes().property_sets.get("minecraft:furnace_input"),
        Some(&vec![42, 43])
    );
    assert_eq!(world.recipes().stonecutter_recipes.len(), 1);
    let world_counters = world.counters();
    assert_eq!(world_counters.update_recipes_packets, 1);
    assert_eq!(world_counters.recipe_property_sets_tracked, 1);
    assert_eq!(world_counters.recipe_property_set_items_tracked, 2);
    assert_eq!(world_counters.stonecutter_recipes_tracked, 1);
}

#[test]
fn client_common_waypoint_events_update_world_and_snapshot_counters() {
    let waypoint_id = Uuid::from_u128(0x00112233445566778899aabbccddeeff);
    let (tx, mut rx) = mpsc::channel(4);
    tx.try_send(NetEvent::CustomPayload(CustomPayload {
        id: "minecraft:brand".to_string(),
        payload: CustomPayloadBody::Brand {
            brand: "vanilla".to_string(),
        },
    }))
    .unwrap();
    tx.try_send(NetEvent::ClearDialog).unwrap();
    tx.try_send(NetEvent::ShowDialog(ShowDialog {
        dialog: DialogHolder::Reference { registry_id: 11 },
    }))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::Waypoint(
        TrackedWaypointPacket {
            operation: WaypointOperation::Track,
            waypoint: TrackedWaypoint {
                identifier: WaypointIdentifier::Uuid(waypoint_id),
                icon: WaypointIcon {
                    style: "minecraft:default".to_string(),
                    color_rgb: Some(0x112233),
                },
                data: WaypointData::Position(WaypointVec3i {
                    x: 10,
                    y: 64,
                    z: -5,
                }),
            },
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        4
    );
    assert_eq!(world.server_brand(), Some("vanilla"));
    assert_eq!(
        world.last_custom_payload(),
        Some(&bbb_world::CustomPayloadState {
            id: "minecraft:brand".to_string(),
            kind: "brand".to_string(),
            brand: Some("vanilla".to_string()),
            raw_payload_len: 0,
        })
    );
    assert_eq!(
        world.current_dialog(),
        Some(&bbb_world::DialogState {
            holder_kind: "reference".to_string(),
            registry_id: Some(11),
            raw_dialog_payload_len: 0,
        })
    );
    let world_counters = world.counters();
    assert_eq!(world_counters.custom_payload_packets, 1);
    assert_eq!(world_counters.custom_payload_brand_packets, 1);
    assert_eq!(world_counters.custom_payload_unknown_packets, 0);
    assert_eq!(world_counters.clear_dialog_packets, 1);
    assert_eq!(world_counters.show_dialog_packets, 1);
    assert_eq!(world_counters.waypoint_packets, 1);
    assert_eq!(world_counters.waypoints_tracked, 1);
    assert_eq!(world_counters.waypoint_updates_applied, 0);
    assert_eq!(world_counters.waypoint_updates_ignored, 0);
    assert_eq!(world_counters.waypoint_untracks_ignored, 0);
    let waypoint_key = format!("uuid:{waypoint_id}");
    let tracked_waypoint = world
        .tracked_waypoints()
        .get(&waypoint_key)
        .expect("tracked waypoint is stored in world");
    assert_eq!(tracked_waypoint.identifier_kind, "uuid");
    assert_eq!(tracked_waypoint.identifier, waypoint_id.to_string());
    assert_eq!(tracked_waypoint.icon_style, "minecraft:default");
    assert_eq!(tracked_waypoint.icon_color_rgb, Some(0x112233));
    assert_eq!(
        tracked_waypoint.data,
        bbb_world::WaypointDataState {
            kind: "position".to_string(),
            position: Some(bbb_world::WaypointVec3iState {
                x: 10,
                y: 64,
                z: -5,
            }),
            chunk: None,
            azimuth: None,
        }
    );
    assert_eq!(
        world.last_waypoint_event(),
        Some(&bbb_world::WaypointEventState {
            operation: "track".to_string(),
            waypoint: tracked_waypoint.clone(),
            applied: true,
        })
    );
}

#[test]
fn player_action_events_update_snapshot_counters() {
    let (tx, mut rx) = mpsc::channel(4);
    tx.try_send(NetEvent::Play(PlayClientbound::PlayerCombatEnter))
        .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::PlayerCombatEnd(
        PlayerCombatEnd { duration: 37 },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::PlayerCombatKill(
        PlayerCombatKill {
            player_id: 123,
            message: "You died".to_string(),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::PlayerLookAt(
        PlayerLookAt {
            from_anchor: EntityAnchor::Eyes,
            position: ProtocolVec3d {
                x: 10.5,
                y: 64.0,
                z: -2.25,
            },
            target: Some(PlayerLookAtTarget {
                entity_id: 456,
                to_anchor: EntityAnchor::Feet,
            }),
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        4
    );
    assert_eq!(world.counters().player_combat_enter_packets, 1);
    assert_eq!(world.counters().player_combat_end_packets, 1);
    assert_eq!(world.counters().player_combat_kill_packets, 1);
    assert_eq!(
        world.last_player_combat(),
        Some(&bbb_world::PlayerCombatEventState {
            kind: "kill".to_string(),
            duration: None,
            player_id: Some(123),
            message: Some("You died".to_string()),
        })
    );
    assert_eq!(world.counters().player_look_at_packets, 1);
    assert_eq!(
        world.local_player().last_look_at,
        Some(bbb_world::LocalPlayerLookAtState {
            from_anchor: EntityAnchor::Eyes,
            position: ProtocolVec3d {
                x: 10.5,
                y: 64.0,
                z: -2.25,
            },
            target_entity_id: Some(456),
            to_anchor: Some(EntityAnchor::Feet),
        })
    );
}
