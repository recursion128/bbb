use super::*;
use crate::inventory::{ContainerAnchor, MenuLayout};
use bbb_protocol::item_types::VANILLA_ITEM_STONE_ID;
use bbb_protocol::packets::ItemStackSummary;

fn stone(count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(VANILLA_ITEM_STONE_ID),
        count,
        component_patch: Default::default(),
    }
}

fn collect(menu: &mut PlayerInventoryMenu, mode: MenuBroadcastMode) -> Vec<MenuBroadcastEvent> {
    let mut cursor = menu.begin_menu_broadcast(mode);
    let mut events = Vec::new();
    while let Some(event) = menu
        .next_menu_broadcast(crate::inventory::test_item_resolver(), None, &mut cursor)
        .unwrap()
    {
        events.push(event);
    }
    events
}

fn settled() -> PlayerInventoryMenu {
    let mut menu = PlayerInventoryMenu::empty();
    menu.send_all_data_to_remote().unwrap();
    menu
}

#[test]
fn menu_non_inventory_slots_and_cursor_do_not_fire_inventory_listener() {
    let mut menu = settled();
    menu.crafting_result = RuntimeItemStack::from_summary(stone(1));
    menu.crafting_grid[0] = RuntimeItemStack::from_summary(stone(2));
    menu.menu.carried = RuntimeItemStack::from_summary(stone(3));
    assert!(collect(&mut menu, MenuBroadcastMode::ListenersOnly).is_empty());
    menu.open_menu(
        crate::inventory::test_item_resolver(),
        None,
        MenuLayout::Generic3x3,
        ContainerAnchor {
            positions: Vec::new(),
        },
        vec![RuntimeItemStack::from_summary(stone(5)); 9],
        false,
    )
    .unwrap()
    .unwrap();
    collect(&mut menu, MenuBroadcastMode::ListenersOnly);
    menu.container[0] = RuntimeItemStack::from_summary(stone(6));
    assert!(collect(&mut menu, MenuBroadcastMode::ListenersOnly).is_empty());
}

#[test]
fn menu_creative_inventory_scope_runs_callbacks_and_restores_open_menu() {
    let mut menu = settled();
    let (opened, _) = menu
        .open_menu(
            crate::inventory::test_item_resolver(),
            None,
            MenuLayout::Generic3x3,
            ContainerAnchor {
                positions: Vec::new(),
            },
            vec![RuntimeItemStack::from_summary(stone(5)); 9],
            false,
        )
        .unwrap()
        .unwrap();
    let parked = menu.begin_inventory_menu();
    assert_eq!(menu.container_id(), 0);
    assert!(menu
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            36,
            RuntimeItemStack::from_summary(stone(1))
        )
        .unwrap());
    menu.set_remote_slot(36, &stone(1));
    let mut cursor = menu.begin_menu_broadcast(MenuBroadcastMode::Changes);
    assert_eq!(
        menu.next_menu_broadcast(crate::inventory::test_item_resolver(), None, &mut cursor)
            .unwrap(),
        Some(MenuBroadcastEvent::InventoryChanged {
            inventory_slot: 0,
            changed_item: RuntimeItemStack::from_summary(stone(1)),
        })
    );
    menu.add_resource(1, &mut RuntimeItemStack::from_summary(stone(2)))
        .unwrap();
    assert_eq!(
        menu.next_menu_broadcast(crate::inventory::test_item_resolver(), None, &mut cursor)
            .unwrap(),
        Some(MenuBroadcastEvent::InventoryChanged {
            inventory_slot: 1,
            changed_item: RuntimeItemStack::from_summary(stone(2)),
        })
    );
    assert!(
        matches!(menu.next_menu_broadcast(crate::inventory::test_item_resolver(), None, &mut cursor).unwrap(), Some(MenuBroadcastEvent::Remote(MenuSync::Slot { container_id: 0, slot: 37, item, .. })) if item == stone(2))
    );
    assert_eq!(
        menu.next_menu_broadcast(crate::inventory::test_item_resolver(), None, &mut cursor)
            .unwrap(),
        None
    );
    menu.end_inventory_menu(parked);
    assert_eq!(menu.container_id(), opened.container_id);
    assert_eq!(
        &*menu.slot_item(0),
        &RuntimeItemStack::from_summary(stone(5))
    );
    assert_eq!(
        &menu.inventory_slots()[..2],
        &[
            RuntimeItemStack::from_summary(stone(1)),
            RuntimeItemStack::from_summary(stone(2))
        ]
    );
    let parked = menu.begin_inventory_menu();
    assert!(collect(&mut menu, MenuBroadcastMode::Changes).is_empty());
    menu.end_inventory_menu(parked);
    assert_eq!(menu.container_id(), opened.container_id);
}

mod item_hash_tests {
    use super::*;
    use crate::connection::{ClientboundPlayPacket, ConnectionId};
    use crate::inventory::RemoteSlot;
    use crate::world::{ServerWorld, WorldCommand, WorldEvent};
    use bbb_facts::container_inputs::ContainerInput;
    use bbb_facts::static_registry_elements::vanilla_static_registry_elements;
    use bbb_protocol::component_literal_nbt;
    use bbb_protocol::content_registry::{ContentRegistryBuilder, ModItemRegistration};
    use bbb_protocol::data_component_types::{
        VANILLA_DATA_COMPONENT_CREATIVE_SLOT_LOCK_TYPE_ID,
        VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID, VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
        VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID, VANILLA_DATA_COMPONENT_RARITY_TYPE_ID,
        VANILLA_DATA_COMPONENT_TRIM_TYPE_ID, VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
    };
    use bbb_protocol::item_types::{
        VANILLA_ITEM_APPLE_ID, VANILLA_ITEM_DIAMOND_CHESTPLATE_ID, VANILLA_ITEM_DIAMOND_SWORD_ID,
        VANILLA_ITEM_IRON_SWORD_ID, VANILLA_ITEM_POTION_ID, VANILLA_ITEM_STICK_ID,
    };
    use bbb_protocol::packets::{
        ContainerClick, DataComponentPatchSummary, HashedComponentPatch, HashedItemStack,
        ItemEnchantmentSummary, ItemStackTemplateSummary, MobEffectInstanceSummary,
        PlayServerbound, PotionContentsSummary,
    };
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
    use bbb_registry_codec::item_hash::hashed_stack_from_summary;
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::Arc;

    const FIRST_HOTBAR: i16 = 36;
    const SECOND_HOTBAR: i16 = 37;

    fn static_holder_id(registry_id: &str, resource_id: &str) -> i32 {
        vanilla_static_registry_elements(registry_id)
            .expect("static registry exists")
            .iter()
            .position(|&entry| entry == resource_id)
            .expect("element exists") as i32
    }

    fn enchanted_sword(resolver: &dyn PersistentItemStackHolderResolver) -> ItemStackSummary {
        let sharpness = resolver
            .holder_id("minecraft:enchantment", "minecraft:sharpness")
            .expect("sharpness enchantment exists");
        let mut patch = DataComponentPatchSummary::default();
        patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID);
        patch.enchantments.push(ItemEnchantmentSummary {
            holder_id: sharpness,
            level: 5,
        });
        ItemStackSummary {
            item_id: Some(VANILLA_ITEM_DIAMOND_SWORD_ID),
            count: 1,
            component_patch: patch.into(),
        }
    }

    fn renamed_sword(name: &str) -> ItemStackSummary {
        let mut patch = DataComponentPatchSummary::default();
        patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID);
        patch.custom_name = Some(name.to_owned());
        patch.custom_name_nbt = Some(component_literal_nbt(name));
        ItemStackSummary {
            item_id: Some(VANILLA_ITEM_DIAMOND_SWORD_ID),
            count: 1,
            component_patch: patch.into(),
        }
    }

    fn healing_potion() -> ItemStackSummary {
        let healing = static_holder_id("minecraft:potion", "minecraft:healing");
        let speed = static_holder_id("minecraft:mob_effect", "minecraft:speed");
        let mut patch = DataComponentPatchSummary::default();
        patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID);
        patch.potion_contents = PotionContentsSummary {
            potion_id: Some(healing),
            custom_color: Some(0x00FF_4466),
            custom_effects: vec![MobEffectInstanceSummary {
                effect_id: speed,
                amplifier: 1,
                duration: 200,
                ambient: false,
                show_particles: true,
                show_icon: true,
                hidden_effect: None,
            }],
            custom_name: Some("elixir".into()),
        };
        ItemStackSummary {
            item_id: Some(VANILLA_ITEM_POTION_ID),
            count: 1,
            component_patch: patch.into(),
        }
    }

    fn trimmed_chestplate(resolver: &dyn PersistentItemStackHolderResolver) -> ItemStackSummary {
        let material = resolver
            .holder_id("minecraft:trim_material", "minecraft:gold")
            .expect("gold trim material exists");
        let pattern = resolver
            .holder_id("minecraft:trim_pattern", "minecraft:sentry")
            .expect("sentry trim pattern exists");
        let mut patch = DataComponentPatchSummary::default();
        patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_TRIM_TYPE_ID);
        patch.armor_trim_material_id = Some(material);
        patch.armor_trim_pattern_id = Some(pattern);
        ItemStackSummary {
            item_id: Some(VANILLA_ITEM_DIAMOND_CHESTPLATE_ID),
            count: 1,
            component_patch: patch.into(),
        }
    }

    fn custom_data_stick(count: i32) -> ItemStackSummary {
        let mut patch = DataComponentPatchSummary::default();
        patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID);
        patch
            .removed_type_ids
            .push(VANILLA_DATA_COMPONENT_RARITY_TYPE_ID);
        patch.custom_data = Some(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "tier".into(),
            value: NbtSummaryValue::Int(3),
        }]));
        ItemStackSummary {
            item_id: Some(VANILLA_ITEM_STICK_ID),
            count,
            component_patch: patch.into(),
        }
    }

    #[test]
    fn patched_stacks_match_hashed_prediction_without_resync() {
        let resolver = crate::inventory::test_item_resolver();
        let lookup = ServerItemHashLookup::new(resolver, ContentRegistry::vanilla());
        let cases = [
            enchanted_sword(resolver),
            renamed_sword("Excalibur"),
            healing_potion(),
            trimmed_chestplate(resolver),
            custom_data_stick(4),
        ];

        for item in cases {
            let mut menu = PlayerInventoryMenu::empty();
            menu.inventory[0] = RuntimeItemStack::from_summary(item.clone());
            menu.send_all_data_to_remote().unwrap();
            let state_before = menu.state_id();

            let expected_hash =
                hashed_stack_from_summary(&lookup, &item).expect("patched stack is hashable");

            // Pickup patched item from hotbar into cursor with exact prediction.
            menu.suppress_remote_updates();
            menu.clicked(
                resolver,
                None,
                FIRST_HOTBAR,
                0,
                ContainerInput::Pickup,
                true,
            )
            .unwrap();
            menu.set_remote_slot_unsafe(FIRST_HOTBAR, &HashedStack::Empty);
            menu.set_remote_carried(&expected_hash);
            menu.resume_remote_updates();

            let sync = menu.broadcast_changes(resolver, None).unwrap();
            assert!(
                sync.is_empty(),
                "matching patched prediction should not trigger resync: {sync:?}"
            );
            assert_eq!(menu.state_id(), state_before);
            assert_eq!(
                menu.menu.remote_carried,
                RemoteSlot::Known(RuntimeItemStack::from_summary(item.clone())),
                "matching hash promotes RemoteSlot::Hashed to Known"
            );

            // Place into second hotbar slot with exact prediction.
            menu.suppress_remote_updates();
            menu.clicked(
                resolver,
                None,
                SECOND_HOTBAR,
                0,
                ContainerInput::Pickup,
                true,
            )
            .unwrap();
            menu.set_remote_slot_unsafe(SECOND_HOTBAR, &expected_hash);
            menu.set_remote_carried(&HashedStack::Empty);
            menu.resume_remote_updates();

            let sync = menu.broadcast_changes(resolver, None).unwrap();
            assert!(
                sync.is_empty(),
                "placing patched item with matching hash should not trigger resync: {sync:?}"
            );
            assert_eq!(menu.state_id(), state_before);
            assert_eq!(
                menu.menu.remote_slots[SECOND_HOTBAR as usize],
                RemoteSlot::Known(RuntimeItemStack::from_summary(item)),
            );
        }
    }

    #[test]
    fn mismatched_hashed_prediction_triggers_resync() {
        let resolver = crate::inventory::test_item_resolver();
        let lookup = ServerItemHashLookup::new(resolver, ContentRegistry::vanilla());

        // 1. Wrong component hash (e.g. different enchantment digest).
        let sword = enchanted_sword(resolver);
        let mut tampered_hash = hashed_stack_from_summary(&lookup, &sword).unwrap();
        if let HashedStack::Item(ref mut item) = tampered_hash {
            let digest = item
                .components
                .added_components
                .get_mut(&VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID)
                .unwrap();
            *digest ^= 1;
        }
        let mut menu = PlayerInventoryMenu::empty();
        menu.inventory[0] = RuntimeItemStack::from_summary(sword.clone());
        menu.send_all_data_to_remote().unwrap();
        menu.suppress_remote_updates();
        menu.clicked(
            resolver,
            None,
            FIRST_HOTBAR,
            0,
            ContainerInput::Pickup,
            true,
        )
        .unwrap();
        menu.set_remote_slot_unsafe(FIRST_HOTBAR, &HashedStack::Empty);
        menu.set_remote_carried(&tampered_hash);
        menu.resume_remote_updates();
        assert_eq!(
            menu.broadcast_changes(resolver, None).unwrap(),
            vec![MenuSync::Cursor {
                item: sword.clone(),
            }]
        );

        // 2. Wrong item ID (iron sword predicted instead of diamond sword).
        let mut wrong_item = hashed_stack_from_summary(&lookup, &sword).unwrap();
        if let HashedStack::Item(ref mut item) = wrong_item {
            item.item_id = VANILLA_ITEM_IRON_SWORD_ID;
        }
        let mut slot = RemoteSlot::Hashed(wrong_item);
        assert!(!slot.matches(&lookup, &RuntimeItemStack::from_summary(sword.clone())));

        // 3. Wrong count.
        let sticks = custom_data_stick(4);
        let mut wrong_count = hashed_stack_from_summary(&lookup, &sticks).unwrap();
        if let HashedStack::Item(ref mut item) = wrong_count {
            item.count = 3;
        }
        let mut slot = RemoteSlot::Hashed(wrong_count);
        assert!(!slot.matches(&lookup, &RuntimeItemStack::from_summary(sticks.clone())));

        // 4. Wrong removed component set.
        let mut wrong_removed = hashed_stack_from_summary(&lookup, &sticks).unwrap();
        if let HashedStack::Item(ref mut item) = wrong_removed {
            item.components.removed_components = BTreeSet::new();
        }
        let mut slot = RemoteSlot::Hashed(wrong_removed);
        assert!(!slot.matches(&lookup, &RuntimeItemStack::from_summary(sticks)));

        // 5. Missing added component (client thinks stack is unpatched).
        let unpatched_hash = HashedStack::Item(HashedItemStack {
            item_id: VANILLA_ITEM_DIAMOND_SWORD_ID,
            count: 1,
            components: HashedComponentPatch::default(),
        });
        let mut slot = RemoteSlot::Hashed(unpatched_hash);
        assert!(!slot.matches(&lookup, &RuntimeItemStack::from_summary(sword)));

        // 6. Unhashable transient component fails closed and triggers resync.
        let mut transient = ItemStackSummary {
            item_id: Some(VANILLA_ITEM_APPLE_ID),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        };
        transient
            .component_patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_CREATIVE_SLOT_LOCK_TYPE_ID);
        transient.component_patch.creative_slot_lock = true;
        let forged_transient = HashedStack::Item(HashedItemStack {
            item_id: VANILLA_ITEM_APPLE_ID,
            count: 1,
            components: HashedComponentPatch {
                added_components: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_CREATIVE_SLOT_LOCK_TYPE_ID,
                    0,
                )]),
                removed_components: BTreeSet::new(),
            },
        });
        let mut slot = RemoteSlot::Hashed(forged_transient);
        assert!(!slot.matches(&lookup, &RuntimeItemStack::from_summary(transient)));
    }

    #[test]
    fn modded_content_registry_item_matches_hashed_prediction() {
        let resolver = crate::inventory::test_item_resolver();
        let mut builder = ContentRegistryBuilder::new();
        builder.register_item(ModItemRegistration {
            resource_id: "modded:crystal_shard".to_owned(),
            max_stack_size: 16,
            is_block_item: false,
        });
        let content_registry = Arc::new(builder.freeze_for_session().unwrap());
        let mod_item_id = content_registry
            .item_by_resource_id("modded:crystal_shard")
            .unwrap()
            .id;

        let mut patch = DataComponentPatchSummary::default();
        patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID);
        patch.use_remainder = Some(Box::new(ItemStackTemplateSummary {
            item_id: mod_item_id,
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        }));
        let apple_with_mod_remainder = ItemStackSummary {
            item_id: Some(VANILLA_ITEM_APPLE_ID),
            count: 1,
            component_patch: patch.into(),
        };

        let lookup = ServerItemHashLookup::new(resolver, &content_registry);
        let expected_hash = hashed_stack_from_summary(&lookup, &apple_with_mod_remainder)
            .expect("modded content registry item hashes cleanly");

        let mut menu = PlayerInventoryMenu::empty();
        menu.set_content_registry(Arc::clone(&content_registry));
        menu.inventory[0] = RuntimeItemStack::from_summary(apple_with_mod_remainder);
        menu.send_all_data_to_remote().unwrap();
        menu.suppress_remote_updates();
        menu.clicked(
            resolver,
            None,
            FIRST_HOTBAR,
            0,
            ContainerInput::Pickup,
            true,
        )
        .unwrap();
        menu.set_remote_slot_unsafe(FIRST_HOTBAR, &HashedStack::Empty);
        menu.set_remote_carried(&expected_hash);
        menu.resume_remote_updates();

        assert!(menu.broadcast_changes(resolver, None).unwrap().is_empty());
    }

    #[test]
    fn container_click_on_server_world_with_patched_stacks_avoids_resync_and_corrects_mismatches() {
        const PLAYER: ConnectionId = ConnectionId(1);
        let (sender, receiver) = crate::tests::mailbox();
        let mut world = ServerWorld::new(crate::tests::datapack_config()).unwrap();
        crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

        let resolver = world.item_stack_resolver();
        let content_registry = Arc::clone(world.plugins().content_registry());
        let lookup = ServerItemHashLookup::new(&resolver, &content_registry);
        let patched_items = [
            enchanted_sword(&resolver),
            renamed_sword("Blade of Parity"),
            healing_potion(),
            trimmed_chestplate(&resolver),
            custom_data_stick(3),
        ];

        for item in &patched_items {
            world.stock_menu_slot_for_test(PLAYER, FIRST_HOTBAR, item.clone());
            let state_before = world.player_inventory_for_test(PLAYER).state_id();
            let expected_hash =
                hashed_stack_from_summary(&lookup, item).expect("live world stack hashes");

            sender
                .try_send(WorldCommand::Play {
                    connection_id: PLAYER,
                    packet: PlayServerbound::ContainerClick(ContainerClick {
                        container_id: 0,
                        state_id: state_before,
                        slot_num: FIRST_HOTBAR,
                        button_num: 0,
                        input: ContainerInput::Pickup,
                        changed_slots: BTreeMap::from([(FIRST_HOTBAR, HashedStack::Empty)]),
                        carried_item: expected_hash.clone(),
                    }),
                })
                .unwrap();
            let tick = world.tick(receiver.drain());
            let resync_packets: Vec<_> = tick
                .events
                .iter()
                .flat_map(WorldEvent::deliveries)
                .filter(|(_, packet)| {
                    matches!(
                        packet,
                        ClientboundPlayPacket::ContainerSetSlot(_)
                            | ClientboundPlayPacket::SetCursorItem(_)
                            | ClientboundPlayPacket::ContainerSetContent(_)
                    )
                })
                .collect();
            assert!(
                resync_packets.is_empty(),
                "matching patched ContainerClick should send no resync packets: {resync_packets:?}"
            );
            assert_eq!(
                world.player_inventory_for_test(PLAYER).state_id(),
                state_before
            );
            assert_eq!(
                world.player_inventory_for_test(PLAYER).carried().summary(),
                item
            );

            // Put the item back into FIRST_HOTBAR with matching prediction.
            sender
                .try_send(WorldCommand::Play {
                    connection_id: PLAYER,
                    packet: PlayServerbound::ContainerClick(ContainerClick {
                        container_id: 0,
                        state_id: state_before,
                        slot_num: FIRST_HOTBAR,
                        button_num: 0,
                        input: ContainerInput::Pickup,
                        changed_slots: BTreeMap::from([(FIRST_HOTBAR, expected_hash)]),
                        carried_item: HashedStack::Empty,
                    }),
                })
                .unwrap();
            let put_back = world.tick(receiver.drain());
            let put_back_resync: Vec<_> = put_back
                .events
                .iter()
                .flat_map(WorldEvent::deliveries)
                .filter(|(_, packet)| {
                    matches!(
                        packet,
                        ClientboundPlayPacket::ContainerSetSlot(_)
                            | ClientboundPlayPacket::SetCursorItem(_)
                            | ClientboundPlayPacket::ContainerSetContent(_)
                    )
                })
                .collect();
            assert!(
                put_back_resync.is_empty(),
                "matching put-back ContainerClick should send no resync packets: {put_back_resync:?}"
            );
        }

        // Now send a mismatched prediction on the enchanted sword and verify resync triggers.
        let sword = enchanted_sword(&resolver);
        world.stock_menu_slot_for_test(PLAYER, FIRST_HOTBAR, sword.clone());
        let state_before = world.player_inventory_for_test(PLAYER).state_id();
        let mut wrong_hash = hashed_stack_from_summary(&lookup, &sword).unwrap();
        if let HashedStack::Item(ref mut item) = wrong_hash {
            *item
                .components
                .added_components
                .get_mut(&VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID)
                .unwrap() ^= 0x55AA;
        }
        sender
            .try_send(WorldCommand::Play {
                connection_id: PLAYER,
                packet: PlayServerbound::ContainerClick(ContainerClick {
                    container_id: 0,
                    state_id: state_before,
                    slot_num: FIRST_HOTBAR,
                    button_num: 0,
                    input: ContainerInput::Pickup,
                    changed_slots: BTreeMap::from([(FIRST_HOTBAR, wrong_hash.clone())]),
                    carried_item: wrong_hash,
                }),
            })
            .unwrap();
        let mismatched = world.tick(receiver.drain());
        let mut saw_slot_resync = false;
        let mut saw_cursor_resync = false;
        for (_, packet) in mismatched.events.iter().flat_map(WorldEvent::deliveries) {
            match packet {
                ClientboundPlayPacket::ContainerSetSlot(slot) if slot.slot == FIRST_HOTBAR => {
                    assert_eq!(slot.item, ItemStackSummary::empty());
                    saw_slot_resync = true;
                }
                ClientboundPlayPacket::SetCursorItem(cursor) => {
                    assert_eq!(cursor.item, sword);
                    saw_cursor_resync = true;
                }
                _ => {}
            }
        }
        assert!(
            saw_slot_resync,
            "mismatched slot hash must trigger ContainerSetSlot"
        );
        assert!(
            saw_cursor_resync,
            "mismatched cursor hash must trigger SetCursorItem"
        );
    }
}
