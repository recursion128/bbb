//! Player interaction packets: attacking, picking a block, using an item,
//! swinging, and the shared entity flags sneaking and sprinting publish.

use super::*;

/// The sounds a tick published, in event order, as `(recipient, resource id)`.
fn sound_broadcasts(tick: &TickResult) -> Vec<(u64, &'static str)> {
    tick.events
        .iter()
        .flat_map(WorldEvent::deliveries)
        .filter_map(|(ConnectionId(recipient), packet)| match packet {
            ClientboundPlayPacket::Sound(update) => {
                assert_eq!(update.source, SoundSource::Players);
                assert_eq!(update.volume, 1.0);
                assert_eq!(update.pitch, 1.0);
                let SoundEventHolder::Reference { registry_id } = update.sound else {
                    panic!("a registered sound is sent by reference: {update:?}");
                };
                Some((
                    recipient,
                    vanilla_sound_event_resource_id(registry_id).expect("a registered sound"),
                ))
            }
            _ => None,
        })
        .collect()
}

fn neutral_sound_broadcasts(tick: &TickResult) -> Vec<(u64, &'static str)> {
    tick.events
        .iter()
        .flat_map(WorldEvent::deliveries)
        .filter_map(|(ConnectionId(recipient), packet)| match packet {
            ClientboundPlayPacket::Sound(update) if update.source == SoundSource::Neutral => {
                assert_eq!(update.volume, 1.0);
                assert_eq!(update.pitch, 1.0);
                let SoundEventHolder::Reference { registry_id } = update.sound else {
                    panic!("a registered sound is sent by reference: {update:?}");
                };
                Some((
                    recipient,
                    vanilla_sound_event_resource_id(registry_id).expect("a registered sound"),
                ))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn an_attack_within_reach_plays_the_no_damage_sound_for_everyone_nearby() {
    // `handleAttack` (:1784-1810) ends in `Player.attack` (Player.java:951),
    // and every player this world creates is creative, so `hurtOrSimulate`
    // refuses and the swing reports itself only through the no-damage sound.
    // `ServerLevel.playSeededSound` (:1023-1045) excludes nobody, so the
    // attacker hears its own miss.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    let bob = entity_id_of(&world, 2);

    let attacked = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity { entity_id: bob }),
    );
    assert_eq!(
        sound_broadcasts(&attacked),
        vec![
            (1, "minecraft:entity.player.attack.nodamage"),
            (2, "minecraft:entity.player.attack.nodamage"),
        ]
    );
}

#[test]
fn a_sprinting_attack_reports_the_knockback_before_the_miss() {
    // `Player.attack` (Player.java:963-968) plays the knockback sound first
    // when a sprinting player swings at full strength, and the miss still
    // follows because nothing here can be hurt.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    let alice = entity_id_of(&world, 1);
    let bob = entity_id_of(&world, 2);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerCommand(PlayerCommand {
            entity_id: alice,
            action: PlayerCommandAction::StartSprinting,
            data: 0,
        }),
    );

    let attacked = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity { entity_id: bob }),
    );
    assert_eq!(
        sound_broadcasts(&attacked),
        vec![
            (1, "minecraft:entity.player.attack.knockback"),
            (2, "minecraft:entity.player.attack.knockback"),
            (1, "minecraft:entity.player.attack.nodamage"),
            (2, "minecraft:entity.player.attack.nodamage"),
        ]
    );

    // The swing restarted the recharge (`Player.onAttack`, Player.java:1819),
    // so the very next tick's attack is no longer at full strength and the
    // knockback half drops out.
    let again = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity { entity_id: bob }),
    );
    assert_eq!(
        sound_broadcasts(&again),
        vec![
            (1, "minecraft:entity.player.attack.nodamage"),
            (2, "minecraft:entity.player.attack.nodamage"),
        ]
    );
}

#[test]
fn attacking_yourself_is_refused_by_disconnecting() {
    // `handleAttack` (:1795-1801) treats the attacker as one of the entities no
    // client may name, and answers with `invalid_entity_attacked`.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let alice = entity_id_of(&world, 1);

    let attacked = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity { entity_id: alice }),
    );
    assert!(sound_broadcasts(&attacked).is_empty());
    assert!(attacked.events.iter().any(|event| matches!(
        event,
        WorldEvent::Close {
            connection_id: ConnectionId(1),
            reason: WorldCloseReason::InvalidEntityAttacked,
        }
    )));
}

/// `ServerGamePacketListenerImpl.handleAttack` rejects ItemEntity and
/// ExperienceOrb before `Player.attack` and disconnects with
/// `invalid_entity_attacked` (ServerGamePacketListenerImpl.java:1795-1801).
#[test]
fn attacking_a_stored_item_or_experience_orb_disconnects() {
    for (entity_type_id, data) in [
        (
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID,
            crate::entity::EntityData::Item(crate::entity::ItemEntityData::new(
                crate::entity::item::plain_stack("minecraft:stone", 1)
                    .expect("stone is a built-in item"),
                0.0,
            )),
        ),
        (
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
            crate::entity::EntityData::ExperienceOrb(crate::entity::ExperienceOrbData::new(1)),
        ),
    ] {
        let (sender, receiver) = mailbox();
        let mut world = spawn_pinned_world(creative_fixture_config());
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        let (entity_id, _) = world.spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            entity_type_id,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
            data,
        );
        let attacked = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::Attack(AttackEntity { entity_id }),
        );
        assert!(
            attacked.events.iter().any(|event| matches!(
                event,
                WorldEvent::Close {
                    connection_id: ConnectionId(1),
                    reason: WorldCloseReason::InvalidEntityAttacked,
                }
            )),
            "{attacked:?}"
        );
        assert!(world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, entity_id)
            .is_some());
    }
}

/// FallingBlockEntity is not on the packet-level invalid list, but its
/// `isAttackable` override is false (FallingBlockEntity.java:105-106), so
/// `Player.attack` returns without damage, sound or disconnect.
#[test]
fn attacking_a_stored_falling_block_is_a_noop() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let (entity_id, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
        crate::entity::EntityData::FallingBlock(crate::entity::FallingBlockData::new(
            0,
            BlockPos { x: 0, y: 64, z: 0 },
        )),
    );
    let attacked = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity { entity_id }),
    );
    assert!(
        !attacked
            .events
            .iter()
            .any(|event| matches!(event, WorldEvent::Close { .. })),
        "{attacked:?}"
    );
    assert!(sound_broadcasts(&attacked).is_empty(), "{attacked:?}");
}

#[test]
fn an_attack_on_nothing_changes_nothing() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    // The first player tick's creative ranges (ServerPlayer.java:610-627) sync next tick.
    world.tick(Vec::new());

    let attacked = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity { entity_id: 4242 }),
    );
    assert!(attacked.events.is_empty(), "{attacked:?}");
}

/// The floor block the spawn stands on, which every pick test reaches.
const PICKED_FLOOR: BlockPos = BlockPos { x: 1, y: 63, z: 1 };

fn held_slot_of(tick: &TickResult) -> Option<i32> {
    tick.events.iter().find_map(|event| match event {
        WorldEvent::Send {
            packet: ClientboundPlayPacket::SetHeldSlot(update),
            ..
        } => Some(update.slot),
        _ => None,
    })
}

#[test]
fn picking_a_block_selects_the_hotbar_slot_that_already_holds_it() {
    use bbb_server_types::item_stack::RuntimeItemStack;

    for (components, expected_slot) in [
        (vec![], 0),
        (
            vec![nbt(
                "minecraft:rarity",
                NbtSummaryValue::String("common".into()),
            )],
            0,
        ),
        (
            vec![nbt(
                "minecraft:rarity",
                NbtSummaryValue::String("uncommon".into()),
            )],
            3,
        ),
        (
            vec![nbt("!minecraft:rarity", NbtSummaryValue::Compound(vec![]))],
            3,
        ),
    ] {
        let (sender, receiver) = mailbox();
        let mut world = spawn_pinned_world(creative_fixture_config());
        join_loaded_stocked(&sender, &receiver, &mut world, 1, "alice", 1);
        let resolver = world.item_stack_resolver();
        let existing = RuntimeItemStack::try_read(
            &NbtSummaryValue::Compound(vec![
                nbt("id", NbtSummaryValue::String("minecraft:stone".into())),
                nbt("count", NbtSummaryValue::Int(64)),
                nbt("components", NbtSummaryValue::Compound(components)),
            ]),
            &resolver,
        )
        .unwrap();
        *world
            .player_inventory_for_test(ConnectionId(1))
            .slot_mut(0)
            .unwrap() = existing;
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::SetCarriedItem { slot: 3 },
        );

        let picked = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::PickItemFromBlock(PickItemFromBlock {
                pos: PICKED_FLOOR,
                include_data: true,
            }),
        );
        assert_eq!(held_slot_of(&picked), Some(expected_slot), "{picked:?}");
        let inventory = &picked.snapshot.players[0].inventory;
        assert_eq!(i32::from(inventory.selected_slot), expected_slot);
        assert_eq!(inventory.slots[0].count, 64);
        let stone = bbb_protocol::item_types::VANILLA_ITEM_STONE_ID;
        assert_eq!(
            inventory
                .slots
                .iter()
                .filter(|stack| stack.item_id == Some(stone))
                .map(|stack| stack.count)
                .sum::<i32>(),
            if expected_slot == 0 { 64 } else { 65 },
        );
        assert!(world
            .player_inventory_for_test(ConnectionId(1))
            .slot(usize::try_from(expected_slot).unwrap())
            .unwrap()
            .same_item_same_effective_components(
                &RuntimeItemStack::from_summary(ItemStackSummary {
                    item_id: Some(stone),
                    count: 1,
                    ..ItemStackSummary::empty()
                }),
                &resolver,
            )
            .unwrap());
    }
}

#[test]
fn picking_a_block_you_do_not_carry_stocks_the_selected_slot() {
    // With the fixture stack gone `findSlotMatchingItem` finds nothing, so
    // `hasInfiniteMaterials` sends the pick through `addAndPickItem`, whose
    // `getSuitableHotbarSlot` (Inventory.java:162-178) starts at the selected
    // slot and stops at the first empty one — which is the selected slot
    // itself.
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: ItemStackSummary::empty(),
        }),
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCarriedItem { slot: 3 },
    );

    let picked = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PickItemFromBlock(PickItemFromBlock {
            pos: PICKED_FLOOR,
            include_data: false,
        }),
    );
    assert_eq!(held_slot_of(&picked), Some(3), "{picked:?}");
    let inventory = &picked.snapshot.players[0].inventory;
    assert_eq!(inventory.selected_slot, 3);
    assert_eq!(
        inventory.slots[3],
        ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        }
    );
    // The slot the pick filled was empty, so the menu has one changed slot to
    // report on top of the held-slot answer.
    assert!(
        picked.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::ContainerSetSlot(update),
                ..
            } if update.slot == 39
        )),
        "{picked:?}"
    );

    let unbreaking = crate::enchantment_facts::builtin_enchantment_registry()
        .iter()
        .position(|resource_id| resource_id == "minecraft:unbreaking")
        .unwrap();
    let mut enchanted = ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_SWORD_ID),
        count: 1,
        ..ItemStackSummary::empty()
    };
    enchanted
        .component_patch
        .added_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID);
    enchanted.component_patch.enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
        holder_id: i32::try_from(unbreaking).unwrap(),
        level: 1,
    }];
    let mut unenchanted = enchanted.clone();
    unenchanted.component_patch.enchantments.clear();
    for slot in 0..9 {
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
                slot_num: 36 + slot,
                item: if slot == 4 {
                    unenchanted.clone()
                } else {
                    enchanted.clone()
                },
            }),
        );
    }

    // With no empty hotbar slot, an explicitly empty enchantment component
    // lets this sword be replaced while the enchanted swords stay put.
    let picked = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PickItemFromBlock(PickItemFromBlock {
            pos: PICKED_FLOOR,
            include_data: false,
        }),
    );
    assert_eq!(held_slot_of(&picked), Some(4), "{picked:?}");
    let inventory = &picked.snapshot.players[0].inventory;
    assert_eq!(inventory.selected_slot, 4);
    assert_eq!(
        inventory.slots[4],
        ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        }
    );
    for slot in [0, 1, 2, 3, 5, 6, 7, 8] {
        assert_eq!(inventory.slots[slot], enchanted);
    }
}

#[test]
fn picking_air_and_picking_out_of_reach_both_change_nothing() {
    // `Item.BY_BLOCK` has no entry for air, so `getCloneItemStack` is empty and
    // `handlePickItemFromBlock` stops before `tryPickItem` — which is what
    // separates it from a pick the reach test refused: neither answers, and
    // neither is allowed to move the selection.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    // The first player tick's creative ranges (ServerPlayer.java:610-627) sync next tick.
    world.tick(Vec::new());
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCarriedItem { slot: 3 },
    );

    for pos in [
        BlockPos { x: 1, y: 66, z: 1 },
        BlockPos { x: 1, y: -64, z: 1 },
    ] {
        let picked = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::PickItemFromBlock(PickItemFromBlock {
                pos,
                include_data: false,
            }),
        );
        assert!(picked.events.is_empty(), "{pos:?} produced {picked:?}");
        assert_eq!(picked.snapshot.players[0].inventory.selected_slot, 3);
    }
}

#[test]
fn picking_a_player_produces_nothing() {
    // Player inherits null getPickResult; profile output needs a creative operator.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let alice = entity_id_of(&world, 1);

    for entity_id in [alice, 4242] {
        let picked = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::PickItemFromEntity(PickItemFromEntity {
                entity_id,
                include_data: true,
            }),
        );
        assert!(picked.events.is_empty(), "{entity_id} produced {picked:?}");
    }
}

#[test]
fn using_an_item_in_the_air_acknowledges_and_turns_the_player() {
    // `handleUseItem` (:1356-1377): the acknowledgement and the rotation snap
    // run before the item does, and stone answers `PASS`, so they are the
    // whole of what a use in the air produces.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded_stocked(&sender, &receiver, &mut world, 1, "alice", 1);

    let used = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::UseItem(UseItem {
            hand: InteractionHand::MainHand,
            sequence: 9,
            // `Mth.wrapDegrees` folds both of these back into (-180, 180].
            y_rot: 450.0,
            x_rot: -30.0,
        }),
    );
    assert!(
        matches!(
            used.events.as_slice(),
            [WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::BlockChangedAck(ack),
            }] if ack.sequence == 9
        ),
        "{used:?}"
    );
    assert_eq!(
        used.snapshot.players[0].rotation,
        PlayerRotation {
            y_rot: 90.0,
            x_rot: -30.0
        }
    );
}

#[test]
fn using_an_empty_hand_leaves_the_rotation_alone() {
    // `handleUseItem` acknowledges first and only then checks the stack, so an
    // empty hand still moves the sequence forward and nothing else.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let before = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ClientTickEnd,
    )
    .snapshot
    .players[0]
        .rotation;

    let used = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::UseItem(UseItem {
            hand: InteractionHand::OffHand,
            sequence: 3,
            y_rot: 45.0,
            x_rot: 10.0,
        }),
    );
    assert!(
        matches!(
            used.events.as_slice(),
            [WorldEvent::Send {
                packet: ClientboundPlayPacket::BlockChangedAck(ack),
                ..
            }] if ack.sequence == 3
        ),
        "{used:?}"
    );
    assert_eq!(used.snapshot.players[0].rotation, before);
}

#[test]
fn a_negative_use_sequence_is_refused_by_disconnecting() {
    // `ackBlockChangesUpTo` (:2270-2276) raises on a negative sequence, which
    // is the same refusal `handleUseItemOn` makes of one.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    let used = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::UseItem(UseItem {
            hand: InteractionHand::MainHand,
            sequence: -1,
            y_rot: 0.0,
            x_rot: 0.0,
        }),
    );
    assert!(used.events.iter().any(|event| matches!(
        event,
        WorldEvent::Close {
            connection_id: ConnectionId(1),
            reason: WorldCloseReason::InvalidBlockInteraction,
        }
    )));
}

#[test]
fn an_interaction_publishes_only_the_secondary_action_flag() {
    // `handleInteract` (:1813-1839) writes the crouch state before it resolves
    // the target, and `Player.interactOn` (Player.java:824-861) passes on the
    // other player, so the flag flush is the whole visible result.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    world.reset_pending_chunks_for_test(ConnectionId(2), &[]);
    world.reconcile_player_tracking_for_test(ConnectionId(2));
    let alice = entity_id_of(&world, 1);
    let bob = entity_id_of(&world, 2);

    let interacted = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Interact(InteractEntity {
            entity_id: bob,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: true,
        }),
    );
    assert_eq!(
        shared_flag_broadcasts(&interacted),
        vec![
            (2, alice, VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN),
            (1, alice, VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN),
        ]
    );
    assert!(sound_broadcasts(&interacted).is_empty());

    // Aiming at nothing still moves the crouch state, because the write comes
    // first.
    let released = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Interact(InteractEntity {
            entity_id: 4242,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    );
    assert_eq!(
        shared_flag_broadcasts(&released),
        vec![(2, alice, 0), (1, alice, 0)]
    );
}

fn spawn_test_armor_stand(world: &mut ServerWorld, position: Vec3d) -> i32 {
    world
        .spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            position,
            crate::entity::EntityData::ArmorStand(
                crate::entity::ArmorStandData::new().expect("armor stand attributes"),
            ),
        )
        .0
}

#[test]
fn an_armor_stand_equipment_swap_writes_the_hand_and_swings_on_the_server() {
    // `handleInteract` writes the packet hand around `Player.interactOn`, and
    // `SUCCESS_SERVER` swings after the interaction (`ServerGamePacketListenerImpl.java:1823-1833`).
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let stand_id = spawn_test_armor_stand(
        &mut world,
        Vec3d {
            x: 2.5,
            y: 64.0,
            z: 0.5,
        },
    );
    world.tick(receiver.drain());
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: ItemStackSummary {
                item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_IRON_HELMET_ID),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        }),
    );

    let interacted = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Interact(InteractEntity {
            entity_id: stand_id,
            hand: InteractionHand::MainHand,
            location: Vec3d {
                x: 0.0,
                y: 1.8,
                z: 0.0,
            },
            using_secondary_action: false,
        }),
    );
    assert_eq!(animations(&interacted), vec![0]);
    let equip_sound = vanilla_sound_event_registry_id("minecraft:item.armor.equip_iron")
        .expect("the iron equip sound is registered");
    let sound_index = interacted
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::Sound(sound),
                    ..
                } if sound.source == SoundSource::Neutral
                    && matches!(sound.sound, SoundEventHolder::Reference { registry_id }
                        if registry_id == equip_sound)
            )
        })
        .expect("equipping the stand publishes the equip sound");
    let swing_index = interacted
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::EntityAnimation(_),
                    ..
                }
            )
        })
        .expect("SUCCESS_SERVER publishes a swing");
    assert!(sound_index < swing_index);
    assert_eq!(
        interacted.snapshot.players[0].inventory.slots[0].item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_IRON_HELMET_ID)
    );
    let stand = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, stand_id)
        .expect("the stand remains live");
    let crate::entity::EntityData::ArmorStand(data) = stand.data() else {
        panic!("the entity remains an armor stand")
    };
    assert!(data
        .living()
        .item_by_slot(bbb_protocol::equipment_slots::EquipmentSlot::Head)
        .is_some());
}

#[test]
fn armor_stand_pass_and_spectator_success_do_not_mutate_or_server_swing() {
    for game_type in [GameType::Creative, GameType::Spectator] {
        let (sender, receiver) = mailbox();
        let mut world = ServerWorld::new(ServerWorldConfig {
            game_type,
            force_game_mode: true,
            ..creative_fixture_config()
        })
        .unwrap();
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        let stand_id = spawn_test_armor_stand(
            &mut world,
            Vec3d {
                x: 2.5,
                y: 64.0,
                z: 0.5,
            },
        );
        world.tick(receiver.drain());
        if game_type == GameType::Creative {
            play(
                &sender,
                &receiver,
                &mut world,
                1,
                PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
                    slot_num: 36,
                    item: ItemStackSummary {
                        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_NAME_TAG_ID),
                        count: 1,
                        component_patch: DataComponentPatchSummary::default().into(),
                    },
                }),
            );
        }
        let before = world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, stand_id)
            .expect("the stand exists")
            .data()
            .clone();
        let interacted = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::Interact(InteractEntity {
                entity_id: stand_id,
                hand: InteractionHand::MainHand,
                location: Vec3d::default(),
                using_secondary_action: false,
            }),
        );
        assert!(animations(&interacted).is_empty(), "{game_type:?}");
        assert_eq!(
            world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, stand_id)
                .expect("the stand remains")
                .data(),
            &before,
            "{game_type:?}"
        );
    }
}

/// `ArmorStand.interact` passes a name tag on (`ArmorStand.java:187-189`), and
/// `Player.interactOn` then lets `NameTagItem.interactLivingEntity` name the
/// stand, as it does a mannequin (`Player.java:841-850`).
#[test]
fn a_named_name_tag_names_an_armor_stand_and_a_mannequin() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig {
        game_type: GameType::Survival,
        force_game_mode: true,
        ..creative_fixture_config()
    });
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let position = Vec3d {
        x: 2.5,
        y: 64.0,
        z: 0.5,
    };
    let stand_id = spawn_test_armor_stand(&mut world, position);
    let (mannequin_id, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
        Vec3d {
            z: -0.5,
            ..position
        },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
            )
            .unwrap(),
        ),
    );
    world.tick(receiver.drain());
    for target in [stand_id, mannequin_id] {
        let given = crate::tests::entities::console_command(
            &sender,
            &receiver,
            &mut world,
            u64::try_from(target).unwrap(),
            r#"give alice name_tag[custom_name="Steve"]"#,
        );
        assert_eq!(
            crate::tests::entities::console_command_answer(&given).0,
            Some(1)
        );
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::Interact(InteractEntity {
                entity_id: target,
                hand: InteractionHand::MainHand,
                location: Vec3d::default(),
                using_secondary_action: false,
            }),
        );
        let named = world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, target)
            .expect("the target remains")
            .custom_name()
            .map(|name| name.text().to_owned());
        assert_eq!(named.as_deref(), Some("Steve"), "{target}");
        assert!(
            world.publication().players[0].inventory.slots[0]
                .item_id
                .is_none(),
            "the tag is spent on {target}"
        );
    }
}

fn spawn_test_item_frame(world: &mut ServerWorld, glow: bool) -> i32 {
    let attached = BlockPos { x: 2, y: 64, z: 0 };
    let (entity_type, data) = if glow {
        (
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID,
            crate::entity::EntityData::GlowItemFrame(crate::entity::ItemFrameData::new(
                attached,
                Direction::North,
            )),
        )
    } else {
        (
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
            crate::entity::EntityData::ItemFrame(crate::entity::ItemFrameData::new(
                attached,
                Direction::North,
            )),
        )
    };
    world
        .spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            entity_type,
            Vec3d {
                x: 2.5,
                y: 64.5,
                z: 0.5,
            },
            data,
        )
        .0
}

fn set_main_hand_item(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    item: ItemStackSummary,
) {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot { slot_num: 36, item }),
    );
}

fn interact_with_test_frame(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    frame_id: i32,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::Interact(InteractEntity {
            entity_id: frame_id,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    )
}

#[test]
fn item_frame_add_emits_player_block_change_then_consumes_only_in_survival() {
    for (game_type, remaining) in [(GameType::Creative, 2), (GameType::Survival, 1)] {
        let (sender, receiver) = mailbox();
        let mut world = spawn_pinned_world(operator_config(&[("alice", 1)]));
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        set_main_hand_item(
            &sender,
            &receiver,
            &mut world,
            ItemStackSummary {
                item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
                count: 2,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        );
        if game_type == GameType::Survival {
            chat_command(&sender, &receiver, &mut world, 1, "gamemode survival");
        }
        let frame_id = spawn_test_item_frame(&mut world, false);
        world.tick(receiver.drain());

        let interacted = interact_with_test_frame(&sender, &receiver, &mut world, frame_id);
        assert_eq!(
            neutral_sound_broadcasts(&interacted),
            [(1, "minecraft:entity.item_frame.add_item")],
            "{game_type:?}"
        );
        assert_eq!(
            interacted.snapshot.players[0].inventory.slots[0].count, remaining,
            "{game_type:?}"
        );
        let frame = world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, frame_id)
            .expect("the frame remains live");
        let crate::entity::EntityData::ItemFrame(frame) = frame.data() else {
            panic!("the entity remains an item frame")
        };
        let framed = frame
            .item()
            .expect("the frame holds the copied item")
            .summary();
        assert_eq!(framed.count, 1);
        let player_id = entity_id_of(&world, 1);
        let (game_events, _) = world.game_events(Java26BuiltinDimension::Overworld);
        assert!(game_events.iter().any(|event| {
            event.game_event == crate::level_event::VanillaGameEvent::BLOCK_CHANGE
                && event.source_entity_id == Some(player_id)
        }));
    }
}

#[test]
fn glow_item_frame_add_and_rotate_publish_exact_sounds() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    set_main_hand_item(
        &sender,
        &receiver,
        &mut world,
        ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        },
    );
    let frame_id = spawn_test_item_frame(&mut world, true);
    world.tick(receiver.drain());

    let added = interact_with_test_frame(&sender, &receiver, &mut world, frame_id);
    assert_eq!(
        neutral_sound_broadcasts(&added),
        [(1, "minecraft:entity.glow_item_frame.add_item")]
    );
    let rotated = interact_with_test_frame(&sender, &receiver, &mut world, frame_id);
    assert_eq!(
        neutral_sound_broadcasts(&rotated),
        [(1, "minecraft:entity.glow_item_frame.rotate_item")]
    );
    let frame = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, frame_id)
        .expect("the frame remains live");
    let crate::entity::EntityData::GlowItemFrame(frame) = frame.data() else {
        panic!("the entity remains a glow item frame")
    };
    assert_eq!(frame.rotation(), 1);
    assert_eq!(rotated.snapshot.players[0].inventory.slots[0].count, 1);
}

#[test]
fn spectator_item_frame_interaction_is_side_effect_free() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    set_main_hand_item(
        &sender,
        &receiver,
        &mut world,
        ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        },
    );
    let frame_id = spawn_test_item_frame(&mut world, false);
    world.tick(receiver.drain());
    interact_with_test_frame(&sender, &receiver, &mut world, frame_id);
    chat_command(&sender, &receiver, &mut world, 1, "gamemode spectator");
    let before_block_changes = world
        .game_events(Java26BuiltinDimension::Overworld)
        .0
        .iter()
        .filter(|event| event.game_event == crate::level_event::VanillaGameEvent::BLOCK_CHANGE)
        .count();

    let spectator = interact_with_test_frame(&sender, &receiver, &mut world, frame_id);
    assert!(neutral_sound_broadcasts(&spectator).is_empty());
    let frame = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, frame_id)
        .expect("the frame remains live");
    let crate::entity::EntityData::ItemFrame(frame) = frame.data() else {
        panic!("the entity remains an item frame")
    };
    assert_eq!(frame.rotation(), 0);
    let after_block_changes = world
        .game_events(Java26BuiltinDimension::Overworld)
        .0
        .iter()
        .filter(|event| event.game_event == crate::level_event::VanillaGameEvent::BLOCK_CHANGE)
        .count();
    assert_eq!(after_block_changes, before_block_changes);
}

#[test]
fn framed_map_without_runtime_saved_data_still_places() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(operator_config(&[("alice", 1)]));
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    set_main_hand_item(
        &sender,
        &receiver,
        &mut world,
        ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_FILLED_MAP_ID),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID,
                ],
                map_id: Some(7),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    );
    chat_command(&sender, &receiver, &mut world, 1, "gamemode survival");
    let frame_id = spawn_test_item_frame(&mut world, false);
    world.tick(receiver.drain());
    let before_events = world.game_events(Java26BuiltinDimension::Overworld).1;

    let placed = interact_with_test_frame(&sender, &receiver, &mut world, frame_id);
    assert_eq!(
        neutral_sound_broadcasts(&placed),
        [(1, "minecraft:entity.item_frame.add_item")]
    );
    let frame = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, frame_id)
        .expect("the frame remains live");
    let crate::entity::EntityData::ItemFrame(frame) = frame.data() else {
        panic!("the entity remains an item frame")
    };
    assert!(frame.item().is_some());
    assert_eq!(placed.snapshot.players[0].inventory.slots[0].count, 0);
    assert_eq!(
        world.game_events(Java26BuiltinDimension::Overworld).1,
        before_events + 1
    );
}

#[test]
fn item_frame_interaction_and_hanging_attack_keep_the_existing_paths() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded_stocked(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded_stocked(&sender, &receiver, &mut world, 2, "bob", 2);
    let alice = entity_id_of(&world, 1);
    let bob = entity_id_of(&world, 2);
    let (frame_id, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
        Vec3d {
            x: 2.5,
            y: 64.5,
            z: 0.5,
        },
        crate::entity::EntityData::ItemFrame(crate::entity::ItemFrameData::new(
            BlockPos { x: 2, y: 64, z: 0 },
            Direction::North,
        )),
    );
    world.tick(receiver.drain());
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: ItemStackSummary {
                item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        }),
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Interact(InteractEntity {
            entity_id: frame_id,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    );
    let frame = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, frame_id)
        .expect("the frame remains live");
    let crate::entity::EntityData::ItemFrame(frame) = frame.data() else {
        panic!("the entity remains an item frame")
    };
    assert!(frame.item().is_some());

    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerCommand(PlayerCommand {
            entity_id: alice,
            action: PlayerCommandAction::StartSprinting,
            data: 0,
        }),
    );

    let attacked = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity {
            entity_id: frame_id,
        }),
    );
    assert!(world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, frame_id)
        .is_some());
    let frame = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, frame_id)
        .unwrap();
    let crate::entity::EntityData::ItemFrame(frame) = frame.data() else {
        unreachable!()
    };
    assert!(frame.item().is_none());
    let remove_sounds: Vec<_> = attacked
        .events
        .iter()
        .filter_map(|event| match event.sent_packet() {
            Some(ClientboundPlayPacket::Sound(sound)) => {
                let SoundEventHolder::Reference { registry_id } = sound.sound else {
                    panic!("the remove-item sound is registered")
                };
                Some((
                    sound.source,
                    vanilla_sound_event_resource_id(registry_id).unwrap(),
                ))
            }
            _ => None,
        })
        .collect();
    assert!(!remove_sounds.is_empty(), "{attacked:?}");
    assert!(remove_sounds.iter().all(|(source, resource_id)| {
        *source == SoundSource::Neutral && *resource_id == "minecraft:entity.item_frame.remove_item"
    }));
    assert!(!attacked
        .events
        .iter()
        .any(|event| matches!(event, WorldEvent::Close { .. })));

    let followup = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity { entity_id: bob }),
    );
    assert_eq!(
        sound_broadcasts(&followup),
        vec![
            (1, "minecraft:entity.player.attack.knockback"),
            (2, "minecraft:entity.player.attack.knockback"),
            (1, "minecraft:entity.player.attack.nodamage"),
            (2, "minecraft:entity.player.attack.nodamage"),
        ]
    );
}

#[test]
fn player_attacks_reach_armor_stand_wobble_break_and_may_build_gates() {
    let stand_position = Vec3d {
        x: 2.5,
        y: 64.0,
        z: 0.5,
    };

    let (sender, receiver) = mailbox();
    let mut config = operator_config(&[("alice", 1)]);
    config.force_game_mode = true;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "defaultgamemode survival",
    );
    let stand_id = spawn_test_armor_stand(&mut world, stand_position);
    for _ in 0..6 {
        world.tick(receiver.drain());
    }
    let alice = entity_id_of(&world, 1);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerCommand(PlayerCommand {
            entity_id: alice,
            action: PlayerCommandAction::StartSprinting,
            data: 0,
        }),
    );
    let wobble = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity {
            entity_id: stand_id,
        }),
    );
    let knockback_index = wobble
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::Sound(sound),
                    ..
                } if matches!(sound.sound, SoundEventHolder::Reference { registry_id }
                    if vanilla_sound_event_resource_id(registry_id)
                        == Some("minecraft:entity.player.attack.knockback"))
            )
        })
        .expect("a full sprinting hit publishes knockback");
    let wobble_index = wobble
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::EntityEvent(EntityEvent {
                        entity_id,
                        event_id: 32,
                    }),
                    ..
                } if *entity_id == stand_id
            )
        })
        .expect("the first survival hit publishes the wobble");
    assert!(knockback_index < wobble_index);
    assert!(world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, stand_id)
        .is_some());

    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let stand_id = spawn_test_armor_stand(&mut world, stand_position);
    world.tick(receiver.drain());
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity {
            entity_id: stand_id,
        }),
    );
    assert!(world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, stand_id)
        .is_none());

    for game_type in [GameType::Adventure, GameType::Spectator] {
        let (sender, receiver) = mailbox();
        let mut world = spawn_pinned_world(ServerWorldConfig {
            game_type,
            force_game_mode: true,
            ..creative_fixture_config()
        });
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        let stand_id = spawn_test_armor_stand(&mut world, stand_position);
        world.tick(receiver.drain());
        let refused = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::Attack(AttackEntity {
                entity_id: stand_id,
            }),
        );
        assert!(world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, stand_id)
            .is_some());
        assert_eq!(
            sound_broadcasts(&refused),
            vec![(1, "minecraft:entity.player.attack.nodamage")],
            "{game_type:?}"
        );
    }
}

/// The shared-flags bytes a tick published, in event order, as
/// `(recipient, entity id, flags)`; other rows such as the pose may share the
/// packet.
fn shared_flag_broadcasts(tick: &TickResult) -> Vec<(u64, i32, i8)> {
    tick.events
        .iter()
        .flat_map(WorldEvent::deliveries)
        .filter_map(|(ConnectionId(recipient), packet)| match packet {
            ClientboundPlayPacket::SetEntityData(update) => {
                let value = update
                    .values
                    .iter()
                    .find(|value| value.data_id == VANILLA_ENTITY_SHARED_FLAGS_DATA_ID)?;
                assert_eq!(value.serializer_id, VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID);
                let EntityDataValueKind::Byte(flags) = value.value else {
                    panic!("the flags item is a byte: {value:?}");
                };
                Some((recipient, update.id, flags))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn sneak_and_sprint_reach_every_tracking_player_and_the_actor() {
    // `handlePlayerCommand` (:1689-1731) and `handlePlayerInput` (:417-424)
    // write shared flags and send nothing; the visible result is whatever
    // `ServerEntity.sendDirtyEntityData` (ServerEntity.java:336-342) flushes at
    // the end of the tick, through `sendToTrackingPlayersAndSelf` — so unlike
    // movement, the acting player is a recipient too, after the tracking set
    // (`ChunkMap.java:1354-1358`).
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    world.reset_pending_chunks_for_test(ConnectionId(2), &[]);
    world.reconcile_player_tracking_for_test(ConnectionId(2));
    let actor = world
        .publication()
        .players
        .iter()
        .find(|player| player.connection_id == ConnectionId(1))
        .expect("alice is connected")
        .entity_id;

    let sprinting = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerCommand(PlayerCommand {
            entity_id: actor,
            action: PlayerCommandAction::StartSprinting,
            data: 0,
        }),
    );
    assert_eq!(
        shared_flag_broadcasts(&sprinting),
        vec![
            (2, actor, VANILLA_ENTITY_SHARED_FLAG_SPRINTING),
            (1, actor, VANILLA_ENTITY_SHARED_FLAG_SPRINTING),
        ]
    );

    // Sneaking is a different bit on the same byte, so the flush carries both.
    let sneaking = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerInput(PlayerInput {
            shift: true,
            ..PlayerInput::default()
        }),
    );
    let both = VANILLA_ENTITY_SHARED_FLAG_SPRINTING | VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN;
    assert_eq!(
        shared_flag_broadcasts(&sneaking),
        vec![(2, actor, both), (1, actor, both)]
    );

    // Nothing changed, so `packDirty` returns null and no packet is written.
    let unchanged = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerInput(PlayerInput {
            shift: true,
            ..PlayerInput::default()
        }),
    );
    assert_eq!(shared_flag_broadcasts(&unchanged), Vec::new());

    let stopped = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerCommand(PlayerCommand {
            entity_id: actor,
            action: PlayerCommandAction::StopSprinting,
            data: 0,
        }),
    );
    assert_eq!(
        shared_flag_broadcasts(&stopped),
        vec![
            (2, actor, VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN),
            (1, actor, VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN),
        ]
    );

    // No player here carries a glider, so `tryToStartFallFlying` fails and
    // `stopFallFlying` sets bit 7 and clears it again. The byte is back where
    // it started, but both writes changed it, so the item is dirty and the
    // unchanged value is published — which is the only reason that call exists.
    let glide = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerCommand(PlayerCommand {
            entity_id: actor,
            action: PlayerCommandAction::StartFallFlying,
            data: 0,
        }),
    );
    assert_eq!(
        shared_flag_broadcasts(&glide),
        vec![
            (2, actor, VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN),
            (1, actor, VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN),
        ]
    );
}

#[test]
fn movement_state_is_ignored_until_the_client_reports_itself_loaded() {
    // Both handlers put their whole body behind `hasClientLoaded`, so a client
    // that has not sent `ServerboundPlayerLoadedPacket` changes nothing — not
    // even the flags the same packets set a moment later.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(1),
            profile: profile("alice", 1),
        },
    )
    .unwrap();
    let joined = world.tick(receiver.drain());
    let actor = joined.snapshot.players[0].entity_id;
    assert!(!joined.snapshot.players[0].client_loaded);

    for packet in [
        PlayServerbound::PlayerCommand(PlayerCommand {
            entity_id: actor,
            action: PlayerCommandAction::StartSprinting,
            data: 0,
        }),
        PlayServerbound::PlayerInput(PlayerInput {
            shift: true,
            ..PlayerInput::default()
        }),
        PlayServerbound::PlayerCommand(PlayerCommand {
            entity_id: actor,
            action: PlayerCommandAction::StartFallFlying,
            data: 0,
        }),
    ] {
        let result = play(&sender, &receiver, &mut world, 1, packet.clone());
        assert_eq!(
            shared_flag_broadcasts(&result),
            Vec::new(),
            "{packet:?} published flags before the client loaded"
        );
        assert_eq!(result.snapshot.players.len(), 1, "{packet:?} closed play");
    }

    let loaded = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerLoaded,
    );
    assert_eq!(shared_flag_broadcasts(&loaded), Vec::new());
    let sprinting = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerCommand(PlayerCommand {
            entity_id: actor,
            action: PlayerCommandAction::StartSprinting,
            data: 0,
        }),
    );
    assert_eq!(
        shared_flag_broadcasts(&sprinting),
        vec![(1, actor, VANILLA_ENTITY_SHARED_FLAG_SPRINTING)]
    );
}

/// `LivingEntity.swing(hand, true)` (LivingEntity.java:1993-2008): the guard at
/// the top drops a swing that is already under way and not yet halfway through
/// its duration, so the second `/swing` in the tick after the first sends no
/// packet -- while the command reports success both times, because
/// `SwingCommand` counts the living targets and never reads the guard
/// (SwingCommand.java:38-43).
#[test]
fn a_second_swing_in_the_next_tick_reports_success_but_sends_no_animation() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);

    let first = chat_command(&sender, &receiver, &mut world, 1, "swing op");
    assert_eq!(
        animations(&first),
        vec![0],
        "the first swing carries the main-hand action"
    );

    // The tick that ran the command also ran `updateSwingTime`, which moved
    // `swingTime` off the `-1` the swing set it to
    // (LivingEntity.java:2145-2155), so the guard now refuses.
    let second = chat_command(&sender, &receiver, &mut world, 1, "swing op offhand");
    assert!(
        animations(&second).is_empty(),
        "the guard should have dropped the second swing: {:?}",
        second.events
    );
    assert_eq!(
        system_chats(&second)
            .iter()
            .map(|(_, text)| text.as_str())
            .collect::<Vec<_>>(),
        vec!["commands.swing.success.single op"],
        "the command reports whatever the guard then decides"
    );

    // Six ticks is `SwingAnimation.DEFAULT.duration()`
    // (SwingAnimation.java:11-12), after which `updateSwingTime` cleared the
    // flag and the next swing is sent again.
    for _ in 0..6 {
        world.tick(receiver.drain());
    }
    let third = chat_command(&sender, &receiver, &mut world, 1, "swing op offhand");
    assert_eq!(
        animations(&third),
        vec![3],
        "the off-hand literal carries SWING_OFF_HAND"
    );
}

/// `LivingEntity.getCurrentSwingDuration` (LivingEntity.java:1978-1986) reads
/// the swinging hand's `swing_animation`: an iron spear's 19 ticks
/// (`Item.java:526`) keep a second swing refused past the default six.
#[test]
fn a_held_spears_swing_animation_lengthens_the_swing_guard() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "item replace entity op weapon.mainhand with minecraft:iron_spear",
    );

    let first = chat_command(&sender, &receiver, &mut world, 1, "swing op");
    assert_eq!(animations(&first), vec![0]);
    for _ in 0..6 {
        world.tick(receiver.drain());
    }
    let early = chat_command(&sender, &receiver, &mut world, 1, "swing op");
    assert!(animations(&early).is_empty(), "{:?}", early.events);
    for _ in 0..2 {
        world.tick(receiver.drain());
    }
    let late = chat_command(&sender, &receiver, &mut world, 1, "swing op");
    assert_eq!(animations(&late), vec![0]);
}

/// `Interaction.skipAttackInteraction` (`Interaction.java:103-105`) fires
/// `player_hurt_entity` for the attacking player, `response` or not.
#[test]
fn attacking_an_interaction_entity_fires_player_hurt_entity() {
    use crate::entity::interaction;
    use crate::tests::parse_advancement;
    use bbb_server_datapack::advancement::AdvancementElement;

    let mut config = operator_config(&[("alice", 1)]);
    let mut startup = (*startup_generation()).clone();
    startup.advancements.advancements.push(AdvancementElement {
        id: "test:hit".into(),
        source_pack: "test".into(),
        advancement: parse_advancement(&serde_json::json!({
            "criteria": {"hit": {
                "trigger": "minecraft:player_hurt_entity",
                "conditions": {"entity": {"type": "minecraft:interaction"}}
            }}
        }))
        .unwrap(),
    });
    startup
        .advancements
        .advancements
        .sort_by(|left, right| left.id.cmp(&right.id));
    config.datapacks = Some(Arc::new(startup));
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    chat_command(&sender, &receiver, &mut world, 1, "gamemode survival");
    let (entity_id, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_INTERACTION_ID,
        Vec3d {
            x: 2.5,
            y: 64.0,
            z: 0.5,
        },
        EntityData::Interaction(interaction::InteractionData::new()),
    );
    world.tick(receiver.drain());
    assert!(!world.player_advancement_is_done(ConnectionId(1), "test:hit"));

    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity { entity_id }),
    );

    assert!(world.player_advancement_is_done(ConnectionId(1), "test:hit"));
}

/// Every `ClientboundAnimatePacket` action a tick sent.
fn animations(tick: &TickResult) -> Vec<u8> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::EntityAnimation(update),
                ..
            } => Some(update.action),
            _ => None,
        })
        .collect()
}

#[test]
fn interaction_entity_use_records_the_packet_hand_and_grants_matching_advancements() {
    use crate::entity::interaction::{self, PlayerAction};
    use crate::tests::parse_advancement;
    use bbb_server_datapack::advancement::AdvancementElement;

    let mut config = operator_config(&[("alice", 1)]);
    let mut startup = (*startup_generation()).clone();
    for (id, target, item, player_type) in [
        (
            "test:main",
            "minecraft:interaction",
            "minecraft:stone",
            "minecraft:player",
        ),
        (
            "test:off",
            "minecraft:interaction",
            "minecraft:apple",
            "minecraft:player",
        ),
        (
            "test:wrong_target",
            "minecraft:cow",
            "minecraft:stone",
            "minecraft:player",
        ),
        (
            "test:wrong_item",
            "minecraft:interaction",
            "minecraft:diamond",
            "minecraft:player",
        ),
        (
            "test:wrong_player",
            "minecraft:interaction",
            "minecraft:stone",
            "minecraft:cow",
        ),
    ] {
        startup.advancements.advancements.push(AdvancementElement {
            id: id.into(),
            source_pack: "test".into(),
            advancement: parse_advancement(&serde_json::json!({
                "criteria": {"use": {
                    "trigger": "minecraft:player_interacted_with_entity",
                    "conditions": {
                        "entity": {"type": target},
                        "item": {"items": item},
                        "player": {"type": player_type}
                    }
                }}
            }))
            .unwrap(),
        });
    }
    startup
        .advancements
        .advancements
        .sort_by(|left, right| left.id.cmp(&right.id));
    config.datapacks = Some(Arc::new(startup));
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    for (slot, item) in [("weapon.mainhand", "stone"), ("weapon.offhand", "apple")] {
        let written = chat_command(&sender, &receiver, &mut world, 1, &format!(
            "item replace entity alice {slot} with minecraft:{item}[minecraft:custom_data={{marker:7}}] 3"
        ));
        assert!(
            system_chats(&written)
                .iter()
                .any(|(_, text)| text.contains("commands.item.entity.set.success.single")),
            "{:?}",
            system_chats(&written)
        );
    }
    chat_command(&sender, &receiver, &mut world, 1, "gamemode survival");
    let (entity_id, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_INTERACTION_ID,
        Vec3d {
            x: 2.5,
            y: 64.0,
            z: 0.5,
        },
        EntityData::Interaction(interaction::InteractionData::new()),
    );
    world.tick(receiver.drain());
    let inventory_before = world
        .save_request(
            1_786_000_002_000,
            202,
            bbb_server_persistence::RegionCompression::Zlib,
        )
        .unwrap()
        .player_patches[0]
        .created
        .inventory
        .clone();
    let mut last_action = None;
    for response in [false, true] {
        interaction::set_response(
            world
                .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, entity_id)
                .unwrap(),
            response,
        );
        for (hand, advancement) in [
            (InteractionHand::MainHand, "test:main"),
            (InteractionHand::OffHand, "test:off"),
        ] {
            let timestamp = world.game_time();
            let before_events = world.game_events(Java26BuiltinDimension::Overworld).1;
            let tick = play(
                &sender,
                &receiver,
                &mut world,
                1,
                PlayServerbound::Interact(InteractEntity {
                    entity_id,
                    hand,
                    location: Vec3d::default(),
                    using_secondary_action: false,
                }),
            );
            let action = Some(PlayerAction {
                player: profile("alice", 1).uuid,
                timestamp,
            });
            let target = world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, entity_id)
                .unwrap();
            assert_eq!(
                interaction::interaction(target).unwrap().interaction(),
                action
            );
            last_action = action;
            assert!(animations(&tick).is_empty());
            assert_eq!(
                world.game_events(Java26BuiltinDimension::Overworld).1,
                before_events
            );
            assert!(
                world.player_advancement_is_done(ConnectionId(1), advancement),
                "{advancement}"
            );
            if !response && hand == InteractionHand::MainHand {
                assert!(!world.player_advancement_is_done(ConnectionId(1), "test:off"));
            }
            for refused in ["test:wrong_target", "test:wrong_item", "test:wrong_player"] {
                assert!(!world.player_advancement_is_done(ConnectionId(1), refused));
            }
        }
    }
    let saved = world
        .save_request(
            1_786_000_002_000,
            202,
            bbb_server_persistence::RegionCompression::Zlib,
        )
        .unwrap();
    assert_eq!(saved.player_patches[0].created.inventory, inventory_before);
    let mut restored = crate::entity::EntityStore::new(0, crate::entity::EntityIdCounter::new());
    for write in saved.entity_writes {
        let chunk =
            Java26EntityChunkData::parse(write.document, write.chunk_x, write.chunk_z).unwrap();
        for id in crate::entity::save::read_entity_chunk(&mut restored, &chunk).unwrap() {
            if let Some(data) = interaction::interaction(restored.get(id).unwrap()) {
                assert_eq!(data.interaction(), last_action);
                return;
            }
        }
    }
    panic!("the saved interaction entity must read back");
}

#[test]
fn interaction_entity_use_rejects_spectators_and_targets_outside_reach_or_border() {
    use crate::entity::interaction;
    use crate::tests::parse_advancement;
    use bbb_server_datapack::advancement::AdvancementElement;

    for (game_type, position, narrow_border) in [
        (
            GameType::Spectator,
            Vec3d {
                x: 2.5,
                y: 64.0,
                z: 0.5,
            },
            false,
        ),
        (
            GameType::Survival,
            Vec3d {
                x: 20.5,
                y: 64.0,
                z: 0.5,
            },
            false,
        ),
        (
            GameType::Survival,
            Vec3d {
                x: 2.5,
                y: 64.0,
                z: 0.5,
            },
            true,
        ),
    ] {
        let (sender, receiver) = mailbox();
        let mut config = operator_config(&[("alice", 1)]);
        config.game_type = game_type;
        config.force_game_mode = true;
        let mut startup = (*startup_generation()).clone();
        startup.advancements.advancements.push(AdvancementElement {
            id: "test:refused_interaction".into(),
            source_pack: "test".into(),
            advancement: parse_advancement(&serde_json::json!({
                "criteria": {"use": {"trigger": "minecraft:player_interacted_with_entity"}}
            }))
            .unwrap(),
        });
        config.datapacks = Some(Arc::new(startup));
        let mut world = spawn_pinned_world(config);
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        world.set_player_position_for_test(
            ConnectionId(1),
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
        );
        if narrow_border {
            chat_command(&sender, &receiver, &mut world, 1, "worldborder center 0 0");
            chat_command(&sender, &receiver, &mut world, 1, "worldborder set 2");
        }
        let (entity_id, _) = world.spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_INTERACTION_ID,
            position,
            EntityData::Interaction(interaction::InteractionData::new()),
        );
        let tick = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::Interact(InteractEntity {
                entity_id,
                hand: InteractionHand::MainHand,
                location: Vec3d::default(),
                using_secondary_action: false,
            }),
        );
        let target = world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, entity_id)
            .unwrap();
        assert_eq!(
            interaction::interaction(target).unwrap().interaction(),
            None
        );
        assert!(animations(&tick).is_empty());
        assert!(!world.player_advancement_is_done(ConnectionId(1), "test:refused_interaction"));
    }
}

fn mob_interaction_config(cases: &[(&str, serde_json::Value, i32)]) -> ServerWorldConfig {
    let mut config = datapack_config();
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    for (id, conditions, experience) in cases {
        startup.advancements.advancements.push(
            bbb_server_datapack::advancement::AdvancementElement {
                id: (*id).into(),
                source_pack: "test".into(),
                advancement: crate::tests::parse_advancement(&serde_json::json!({
                    "criteria":{"interact":{
                        "trigger":"minecraft:player_interacted_with_entity",
                        "conditions":conditions
                    }},
                    "rewards":{"experience":experience}
                }))
                .unwrap(),
            },
        );
    }
    config
}

fn interaction_mob(world: &mut ServerWorld, resource_id: &str) -> i32 {
    let kind = bbb_protocol::entity_types::vanilla_entity_type_for_resource_id(resource_id)
        .unwrap()
        .id;
    let position = world.player_movement_validation_for_test(ConnectionId(1)).0;
    let mut data = crate::entity::GeneralLivingData::new(kind).unwrap();
    data.set_no_ai(true);
    world
        .spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            kind,
            Vec3d {
                x: position.x + 1.0,
                ..position
            },
            EntityData::GeneralLiving(data),
        )
        .0
}

#[test]
fn mob_interaction_advancements_match_original_food_in_either_hand_and_reward_once() {
    use crate::tests::using_item::{Fixture, ALICE};
    use serde_json::json;

    let condition = json!({
        "player":{"type":"minecraft:player"},
        "entity":{"type":"minecraft:cow"},
        "item":{"items":"minecraft:wheat","count":3,
            "components":{"minecraft:custom_data":{"marked":17}}}
    });
    let mut wrong_item = condition.clone();
    wrong_item["item"]["items"] = json!("minecraft:apple");
    let mut wrong_entity = condition.clone();
    wrong_entity["entity"]["type"] = json!("minecraft:pig");
    let mut wrong_player = condition.clone();
    wrong_player["player"]["type"] = json!("minecraft:cow");
    let mut wrong_component = condition.clone();
    wrong_component["item"]["components"]["minecraft:custom_data"]["marked"] = json!(18);
    let config = mob_interaction_config(&[
        ("test:feeding", condition, 7),
        ("test:wrong_item", wrong_item, 0),
        ("test:wrong_entity", wrong_entity, 0),
        ("test:wrong_player", wrong_player, 0),
        ("test:wrong_component", wrong_component, 0),
        ("test:any", json!({}), 0),
    ]);
    for (hand, slot) in [
        (InteractionHand::MainHand, "weapon.mainhand"),
        (InteractionHand::OffHand, "weapon.offhand"),
    ] {
        let mut fixture = Fixture::with_config(GameType::Survival, config.clone());
        fixture.world.tick_rate_mut().set_frozen(true);
        let cow = interaction_mob(&mut fixture.world, "minecraft:cow");
        let interact = PlayServerbound::Interact(InteractEntity {
            entity_id: cow,
            hand,
            location: Vec3d::default(),
            using_secondary_action: false,
        });
        fixture.command(&format!("item replace entity alice {slot} with stone 3"));
        fixture.play(ALICE, interact.clone());
        assert!(!fixture
            .world
            .player_advancement_is_done(ConnectionId(ALICE), "test:any"));
        let written = fixture.command(&format!(
            "item replace entity alice {slot} with wheat[custom_data={{marked:17b}}] 3"
        ));
        assert!(
            system_chats(&written)
                .iter()
                .any(|(_, text)| text.contains("commands.item.entity.set.success.single")),
            "{:?}",
            system_chats(&written)
        );
        fixture.play(ALICE, interact.clone());
        assert!(fixture
            .world
            .player_advancement_is_done(ConnectionId(ALICE), "test:feeding"));
        assert!(fixture
            .world
            .player_advancement_is_done(ConnectionId(ALICE), "test:any"));
        for id in [
            "test:wrong_item",
            "test:wrong_entity",
            "test:wrong_player",
            "test:wrong_component",
        ] {
            assert!(
                !fixture
                    .world
                    .player_advancement_is_done(ConnectionId(ALICE), id),
                "{id}"
            );
        }
        let publication = fixture.world.publication();
        let inventory = &publication
            .players
            .iter()
            .find(|player| player.connection_id == ConnectionId(ALICE))
            .unwrap()
            .inventory;
        let held = if hand == InteractionHand::MainHand {
            &inventory.slots[0]
        } else {
            &inventory.slots[40]
        };
        assert_eq!(held.count, 2);
        fixture.play(ALICE, interact.clone());
        let second_cow = interaction_mob(&mut fixture.world, "minecraft:cow");
        fixture.play(
            ALICE,
            PlayServerbound::Interact(InteractEntity {
                entity_id: second_cow,
                hand,
                location: Vec3d::default(),
                using_secondary_action: false,
            }),
        );
        assert_eq!(
            fixture.world.experience_total_for_test(ConnectionId(ALICE)),
            7
        );
    }
}

#[test]
fn wolf_sitting_uses_empty_advancement_item_while_cat_and_consumed_sheep_keep_it() {
    use crate::tests::using_item::{Fixture, ALICE};
    use serde_json::json;

    let config = mob_interaction_config(&[
        (
            "test:empty_wolf",
            json!({"entity":{"type":"minecraft:wolf"},"item":{"count":0}}),
            3,
        ),
        (
            "test:wolf_stone",
            json!({"entity":{"type":"minecraft:wolf"},"item":{"items":"minecraft:stone"}}),
            0,
        ),
        (
            "test:cat_stone",
            json!({"entity":{"type":"minecraft:cat"},"item":{"items":"minecraft:stone","count":3}}),
            5,
        ),
        (
            "test:consumed_sheep",
            json!({"entity":{"type":"minecraft:sheep"},"item":{"items":"minecraft:shears"}}),
            7,
        ),
    ]);
    let mut fixture = Fixture::with_config(GameType::Survival, config);
    fixture.world.tick_rate_mut().set_frozen(true);
    fixture.command("item replace entity alice weapon.mainhand with stone 3");
    for kind in ["minecraft:wolf", "minecraft:cat"] {
        let mob = interaction_mob(&mut fixture.world, kind);
        let bbb_protocol::NbtSummaryValue::Compound(mut entries) = fixture
            .world
            .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, mob)
            .unwrap()
        else {
            panic!("a mob record is a compound")
        };
        entries.retain(|entry| entry.name != "Owner");
        let uuid = profile("alice", 1).uuid.as_u128();
        entries.push(bbb_protocol::NbtSummaryEntry {
            name: "Owner".into(),
            value: bbb_protocol::NbtSummaryValue::IntArray(
                (0..4)
                    .map(|word| (uuid >> (96 - 32 * word)) as u32 as i32)
                    .collect(),
            ),
        });
        fixture.world.load_stored_entity_record_for_test(
            Java26BuiltinDimension::Overworld,
            mob,
            &entries,
        );
        let interact = PlayServerbound::Interact(InteractEntity {
            entity_id: mob,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        });
        fixture.play(ALICE, interact.clone());
        fixture.play(ALICE, interact);
    }
    fixture.command("item replace entity alice weapon.mainhand with shears");
    let sheep = interaction_mob(&mut fixture.world, "minecraft:sheep");
    let uuid = fixture
        .world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, sheep)
        .unwrap()
        .uuid();
    fixture.command(&format!("data merge entity {uuid} {{Sheared:1b}}"));
    fixture.play(
        ALICE,
        PlayServerbound::Interact(InteractEntity {
            entity_id: sheep,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    );
    for id in ["test:empty_wolf", "test:cat_stone", "test:consumed_sheep"] {
        assert!(
            fixture
                .world
                .player_advancement_is_done(ConnectionId(ALICE), id),
            "{id}"
        );
    }
    assert!(!fixture
        .world
        .player_advancement_is_done(ConnectionId(ALICE), "test:wolf_stone"));
    assert_eq!(
        fixture.world.experience_total_for_test(ConnectionId(ALICE)),
        15
    );
}

#[test]
fn mounting_mob_advancement_reads_the_completed_player_vehicle_relation() {
    use crate::tests::using_item::{Fixture, ALICE};
    use serde_json::json;

    let config = mob_interaction_config(&[(
        "test:mounted",
        json!({
            "player":{"vehicle":{"type":"minecraft:horse"}},
            "entity":{"type":"minecraft:horse"}
        }),
        7,
    )]);
    let mut fixture = Fixture::with_config(GameType::Survival, config);
    fixture.world.tick_rate_mut().set_frozen(true);
    let horse = interaction_mob(&mut fixture.world, "minecraft:horse");
    let uuid = fixture
        .world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, horse)
        .unwrap()
        .uuid();
    fixture.command(&format!("data merge entity {uuid} {{Tame:1b}}"));
    fixture.play(
        ALICE,
        PlayServerbound::Interact(InteractEntity {
            entity_id: horse,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    );
    assert!(fixture
        .world
        .player_advancement_is_done(ConnectionId(ALICE), "test:mounted"));
    assert_eq!(
        fixture.world.experience_total_for_test(ConnectionId(ALICE)),
        7
    );
}

#[test]
fn unleashing_a_mob_uses_an_empty_advancement_item() {
    use crate::tests::using_item::{Fixture, ALICE};
    use serde_json::json;

    let config = mob_interaction_config(&[
        (
            "test:unleashed",
            json!({
                "entity":{"type":"minecraft:cow"}, "item":{"count":0}
            }),
            7,
        ),
        ("test:stone", json!({"item":{"items":"minecraft:stone"}}), 0),
    ]);
    let mut fixture = Fixture::with_config(GameType::Survival, config);
    fixture.world.tick_rate_mut().set_frozen(true);
    let cow = interaction_mob(&mut fixture.world, "minecraft:cow");
    let interact = PlayServerbound::Interact(InteractEntity {
        entity_id: cow,
        hand: InteractionHand::MainHand,
        location: Vec3d::default(),
        using_secondary_action: false,
    });
    fixture.command("item replace entity alice weapon.mainhand with lead");
    fixture.play(ALICE, interact.clone());
    assert!(!fixture
        .world
        .player_advancement_is_done(ConnectionId(ALICE), "test:unleashed"));
    fixture.command("item replace entity alice weapon.mainhand with stone");
    fixture.play(ALICE, interact);
    assert!(fixture
        .world
        .player_advancement_is_done(ConnectionId(ALICE), "test:unleashed"));
    assert!(!fixture
        .world
        .player_advancement_is_done(ConnectionId(ALICE), "test:stone"));
    assert_eq!(
        fixture.world.experience_total_for_test(ConnectionId(ALICE)),
        7
    );
}

#[test]
fn poisoned_parrot_interaction_advancement_reads_the_applied_effect() {
    use crate::tests::using_item::{Fixture, ALICE};
    use serde_json::json;

    let config = mob_interaction_config(&[(
        "test:poisoned_parrot",
        json!({
            "entity":{"type":"minecraft:parrot","effects":{"minecraft:poison":{}}},
            "item":{"items":"minecraft:cookie","count":3}
        }),
        7,
    )]);
    let mut fixture = Fixture::with_config(GameType::Survival, config);
    fixture.world.tick_rate_mut().set_frozen(true);
    let parrot = interaction_mob(&mut fixture.world, "minecraft:parrot");
    let uuid = fixture
        .world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, parrot)
        .unwrap()
        .uuid();
    fixture.command(&format!("data merge entity {uuid} {{Invulnerable:1b}}"));
    fixture.command("item replace entity alice weapon.mainhand with cookie 3");
    fixture.play(
        ALICE,
        PlayServerbound::Interact(InteractEntity {
            entity_id: parrot,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    );
    assert!(fixture
        .world
        .player_advancement_is_done(ConnectionId(ALICE), "test:poisoned_parrot"));
    assert!(fixture
        .world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, parrot)
        .is_some());
    assert_eq!(
        fixture.world.experience_total_for_test(ConnectionId(ALICE)),
        7
    );
}

#[test]
fn nonliving_equipment_interactions_award_original_stack_only_after_success() {
    use crate::tests::using_item::{Fixture, ALICE};
    use serde_json::json;

    for stand in [false, true] {
        let kind = if stand {
            "minecraft:armor_stand"
        } else {
            "minecraft:item_frame"
        };
        let item = if stand { "iron_helmet" } else { "wheat" };
        let count = if stand { 1 } else { 3 };
        let config = mob_interaction_config(&[
            (
                "test:equipped",
                json!({
                    "entity":{"type":kind},
                    "item":{"items":format!("minecraft:{item}"),"count":count,
                        "components":{"minecraft:custom_data":{"marked":17}}}
                }),
                7,
            ),
            ("test:any_equipment_interaction", json!({}), 0),
        ]);
        let mut fixture = Fixture::with_config(GameType::Survival, config);
        fixture.world.tick_rate_mut().set_frozen(true);
        fixture.world.set_player_position_for_test(
            ConnectionId(ALICE),
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
        );
        let target = if stand {
            spawn_test_armor_stand(
                &mut fixture.world,
                Vec3d {
                    x: 2.5,
                    y: 64.0,
                    z: 0.5,
                },
            )
        } else {
            spawn_test_item_frame(&mut fixture.world, false)
        };
        let packet = PlayServerbound::Interact(InteractEntity {
            entity_id: target,
            hand: InteractionHand::OffHand,
            location: Vec3d {
                x: 0.0,
                y: 1.8,
                z: 0.0,
            },
            using_secondary_action: false,
        });
        fixture.play(ALICE, packet.clone());
        assert!(!fixture
            .world
            .player_advancement_is_done(ConnectionId(ALICE), "test:equipped"));
        assert!(!fixture
            .world
            .player_advancement_is_done(ConnectionId(ALICE), "test:any_equipment_interaction"));
        fixture.command(&format!(
            "item replace entity alice weapon.offhand with {item}[custom_data={{marked:17b}}] {count}",
        ));
        let abilities = fixture.world.player_abilities_for_test(ConnectionId(ALICE));
        fixture.world.set_player_mode_and_abilities_for_test(
            ConnectionId(ALICE),
            GameType::Spectator,
            abilities,
        );
        fixture.play(ALICE, packet.clone());
        assert!(!fixture
            .world
            .player_advancement_is_done(ConnectionId(ALICE), "test:equipped"));
        assert!(!fixture
            .world
            .player_advancement_is_done(ConnectionId(ALICE), "test:any_equipment_interaction"));
        fixture.world.set_player_mode_and_abilities_for_test(
            ConnectionId(ALICE),
            GameType::Survival,
            abilities,
        );
        fixture.play(ALICE, packet);
        assert!(
            fixture
                .world
                .player_advancement_is_done(ConnectionId(ALICE), "test:equipped"),
            "{kind}"
        );
        assert_eq!(
            fixture
                .world
                .player_inventory_for_test(ConnectionId(ALICE))
                .off_hand_stack()
                .count(),
            count - 1
        );
    }
}

#[test]
fn boat_interaction_advancements_read_mount_relation_and_empty_unleashing_item() {
    use crate::tests::using_item::{Fixture, ALICE};
    use serde_json::json;
    let config = mob_interaction_config(&[
        (
            "test:mounted_boat",
            json!({"entity":{"type":"minecraft:oak_boat"},
            "player":{"vehicle":{"type":"minecraft:oak_boat"}}}),
            7,
        ),
        (
            "test:unleashed_boat",
            json!({"entity":{"type":"minecraft:oak_boat"},"item":{"count":0}}),
            0,
        ),
    ]);
    let mut fixture = Fixture::with_config(GameType::Survival, config);
    fixture.world.tick_rate_mut().set_frozen(true);
    let position = fixture
        .world
        .player_movement_validation_for_test(ConnectionId(ALICE))
        .0;
    let boat = fixture
        .world
        .spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
            position,
            EntityData::Boat(crate::entity::BoatData::new()),
        )
        .0;
    let packet = PlayServerbound::Interact(InteractEntity {
        entity_id: boat,
        hand: InteractionHand::MainHand,
        location: Vec3d::default(),
        using_secondary_action: false,
    });
    fixture.command("item replace entity alice weapon.mainhand with lead");
    fixture.play(ALICE, packet.clone());
    assert!(!fixture
        .world
        .player_advancement_is_done(ConnectionId(ALICE), "test:mounted_boat"));
    fixture.command("item replace entity alice weapon.mainhand with stone");
    fixture.play(ALICE, packet.clone());
    assert!(fixture
        .world
        .player_advancement_is_done(ConnectionId(ALICE), "test:unleashed_boat"));
    fixture.play(ALICE, packet);
    assert!(fixture
        .world
        .player_advancement_is_done(ConnectionId(ALICE), "test:mounted_boat"));
}

#[test]
fn vehicle_interaction_advancements_include_empty_wrong_fuel_and_cargo_use() {
    use crate::tests::using_item::{Fixture, ALICE};
    use serde_json::json;
    for (kind, data, secondary, item) in [
        (
            "minecraft:furnace_minecart",
            EntityData::FurnaceMinecart(crate::entity::FurnaceMinecartData::new()),
            false,
            None,
        ),
        (
            "minecraft:furnace_minecart",
            EntityData::FurnaceMinecart(crate::entity::FurnaceMinecartData::new()),
            false,
            Some("stone"),
        ),
        (
            "minecraft:chest_minecart",
            EntityData::ChestMinecart(crate::entity::MinecartContainerData::new_chest()),
            false,
            Some("stone"),
        ),
        (
            "minecraft:oak_chest_boat",
            EntityData::ChestBoat(crate::entity::ChestBoatData::new()),
            true,
            Some("stone"),
        ),
    ] {
        let config = mob_interaction_config(&[(
            "test:vehicle",
            json!({"entity":{"type":kind},"item":{"count":if item.is_some(){3}else{0}}}),
            0,
        )]);
        let mut fixture = Fixture::with_config(GameType::Survival, config);
        fixture.world.tick_rate_mut().set_frozen(true);
        let position = fixture
            .world
            .player_movement_validation_for_test(ConnectionId(ALICE))
            .0;
        let target = fixture
            .world
            .spawn_stored_entity_for_test(
                Java26BuiltinDimension::Overworld,
                bbb_protocol::entity_types::vanilla_entity_type_for_resource_id(kind)
                    .unwrap()
                    .id,
                position,
                data,
            )
            .0;
        if let Some(item) = item {
            fixture.command(&format!(
                "item replace entity alice weapon.mainhand with {item} 3"
            ));
        }
        let packet = PlayServerbound::Interact(InteractEntity {
            entity_id: target,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: secondary,
        });
        let abilities = fixture.world.player_abilities_for_test(ConnectionId(ALICE));
        fixture.world.set_player_mode_and_abilities_for_test(
            ConnectionId(ALICE),
            GameType::Spectator,
            abilities,
        );
        fixture.play(ALICE, packet.clone());
        assert!(!fixture
            .world
            .player_advancement_is_done(ConnectionId(ALICE), "test:vehicle"));
        fixture.world.set_player_mode_and_abilities_for_test(
            ConnectionId(ALICE),
            GameType::Survival,
            abilities,
        );
        fixture.play(ALICE, packet);
        assert!(
            fixture
                .world
                .player_advancement_is_done(ConnectionId(ALICE), "test:vehicle"),
            "{kind} {item:?}"
        );
    }
}

/// `getCloneItemStack` overrides (CropBlock.java:169-171, FlowerPotBlock.java:104-106,
/// CandleCakeBlock.java:111-113, BannerBlockEntity.java:78-82) and creative
/// Ctrl+pick's `addBlockDataToItem` (ServerGamePacketListenerImpl.java:711-722).
#[test]
fn picking_override_blocks_selects_their_clone_item() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(operator_config(&[("alice", 1)]));
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let pos = BlockPos { x: 1, y: 64, z: 1 };
    let resolver = world.item_stack_resolver();
    let pick = |world: &mut ServerWorld, block: &str, include_data: bool| {
        chat_command(
            &sender,
            &receiver,
            world,
            1,
            &format!("setblock 1 64 1 {block}"),
        );
        play(
            &sender,
            &receiver,
            world,
            1,
            PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
                slot_num: 36,
                item: ItemStackSummary::empty(),
            }),
        );
        play(
            &sender,
            &receiver,
            world,
            1,
            PlayServerbound::PickItemFromBlock(PickItemFromBlock { pos, include_data }),
        );
        world
            .player_runtime_inventory_slot_for_test(ConnectionId(1), 0)
            .write(&resolver)
            .unwrap()
    };
    let id = |saved: &Option<NbtSummaryValue>| match saved {
        Some(NbtSummaryValue::Compound(entries)) => entries
            .iter()
            .find(|entry| entry.name == "id")
            .map(|entry| entry.value.clone()),
        _ => None,
    };
    for (block, item) in [
        ("wheat[age=3]", "minecraft:wheat_seeds"),
        ("potted_dandelion", "minecraft:dandelion"),
        ("potted_azalea_bush", "minecraft:azalea"),
        ("candle_cake", "minecraft:cake"),
        ("pumpkin_stem", "minecraft:pumpkin_seeds"),
    ] {
        assert_eq!(
            id(&pick(&mut world, block, false)),
            Some(NbtSummaryValue::String(item.into())),
            "{block}"
        );
    }
    assert_eq!(pick(&mut world, "nether_portal", false), None);

    let banner = "white_banner{patterns:[{color:\"red\",pattern:\"minecraft:stripe_top\"}]}";
    let saved = format!("{:?}", pick(&mut world, banner, false));
    assert!(saved.contains("minecraft:banner_patterns"), "{saved}");
    assert!(saved.contains("stripe_top"), "{saved}");

    let pot = "decorated_pot{sherds:[\"minecraft:brick\",\"minecraft:angler_pottery_sherd\",\"minecraft:brick\",\"minecraft:brick\"]}";
    let saved = format!("{:?}", pick(&mut world, pot, true));
    assert!(saved.contains("angler_pottery_sherd"), "{saved}");
}
