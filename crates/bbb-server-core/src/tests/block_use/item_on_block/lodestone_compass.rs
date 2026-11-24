use super::*;
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID, VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
};
use bbb_protocol::packets::data_components::LodestoneTargetSummary;

fn click(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    pos: BlockPos,
    hand: InteractionHand,
    sequence: i32,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::UseItemOn(UseItemOn {
            hand,
            hit: BlockHitResult {
                pos,
                direction: Direction::Up,
                cursor_x: 0.5,
                cursor_y: 0.5,
                cursor_z: 0.5,
                inside: false,
                world_border_hit: false,
            },
            sequence,
        }),
    )
}

fn assert_bound(stack: &ItemStackSummary, pos: BlockPos) {
    assert_eq!(stack.item_id, Some(item_id("minecraft:compass")));
    assert_eq!(stack.count, 1);
    assert_eq!(stack.component_patch.lodestone_tracked, Some(true));
    assert_eq!(
        stack.component_patch.lodestone_target,
        Some(LodestoneTargetSummary {
            dimension: "minecraft:overworld".into(),
            pos,
        })
    );
}

#[test]
fn single_compass_rebinds_in_either_hand_and_keeps_components_on_save_reload() {
    for (hand, slot) in [
        (InteractionHand::MainHand, 0),
        (InteractionHand::OffHand, 40),
    ] {
        let (sender, receiver, mut world) = survival_world_holding("minecraft:compass", 1);
        let resolver = crate::world::registry_runtime::item_stack_resolver_for_test();
        let original = RuntimeItemStack::from_summary(ItemStackSummary {
            item_id: Some(item_id("minecraft:compass")),
            count: 1,
            ..ItemStackSummary::empty()
        })
        .with_component_nbt(
            VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
            Some(&NbtSummaryValue::String("North star".into())),
            resolver,
        )
        .unwrap()
        .with_component_nbt(
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
            Some(&NbtSummaryValue::Compound(vec![nbt(
                "owner",
                NbtSummaryValue::Int(17),
            )])),
            resolver,
        )
        .unwrap()
        .into_summary();
        world.set_inventory_slot_for_test(ConnectionId(1), 0, ItemStackSummary::empty());
        world.set_inventory_slot_for_test(ConnectionId(1), slot, original.clone());
        let stone = BlockPos { x: 0, y: 64, z: 1 };
        let first = BlockPos { x: 1, y: 64, z: 1 };
        let second = BlockPos { x: -1, y: 64, z: 1 };
        place(&mut world, stone, default_state("minecraft:stone"));
        place(&mut world, first, default_state("minecraft:lodestone"));
        place(&mut world, second, default_state("minecraft:lodestone"));
        click(&sender, &receiver, &mut world, stone, hand, 1);
        assert_eq!(
            world.publication().players[0].inventory.slots[slot],
            original
        );
        assert_eq!(items_used(&mut world, "minecraft:compass"), 0);
        let tick = click(&sender, &receiver, &mut world, first, hand, 2);
        assert_bound(&world.publication().players[0].inventory.slots[slot], first);
        let sound = tick
            .events
            .iter()
            .find_map(|event| match event {
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::Sound(sound),
                    ..
                } if sound.sound
                    == (SoundEventHolder::Reference {
                        registry_id: vanilla_sound_event_registry_id(
                            "minecraft:item.lodestone_compass.lock",
                        )
                        .unwrap(),
                    }) =>
                {
                    Some(sound)
                }
                _ => None,
            })
            .expect("binding plays its sound");
        assert_eq!(sound.source, SoundSource::Players);
        assert_eq!((sound.volume, sound.pitch), (1.0, 1.0));
        click(&sender, &receiver, &mut world, second, hand, 3);
        let bound = world.publication().players[0].inventory.slots[slot].clone();
        assert_bound(&bound, second);
        for type_id in [
            VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
        ] {
            assert_eq!(
                RuntimeItemStack::from_summary(bound.clone())
                    .component_nbt(type_id, resolver)
                    .unwrap(),
                RuntimeItemStack::from_summary(original.clone())
                    .component_nbt(type_id, resolver)
                    .unwrap()
            );
        }
        assert_eq!(items_used(&mut world, "minecraft:compass"), 2);
        let request = world
            .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
            .unwrap();
        let patch = &request.player_patches[0];
        let raw = bbb_server_persistence::created_player_document(patch, resolver).unwrap();
        let player = Java26PlayerData::parse(raw, patch.uuid).unwrap();
        let candidate = restore_candidate(4_200, None, vec![(patch.uuid, player)]);
        let mut restored = ServerWorld::restore(datapack_config(), &candidate).unwrap();
        place(&mut restored, first, default_state("minecraft:lodestone"));
        place(&mut restored, second, default_state("minecraft:lodestone"));
        let (sender, receiver) = mailbox();
        join_loaded(&sender, &receiver, &mut restored, 1, "alice", 1);
        assert_eq!(
            restored.publication().players[0].inventory.slots[slot],
            bound
        );
        place(&mut restored, second, default_state("minecraft:air"));
        let tick = restored.tick(receiver.drain());
        let cleared = restored.publication().players[0].inventory.slots[slot].clone();
        let mut expected = bound;
        expected.component_patch.lodestone_target = None;
        assert_eq!(cleared, expected);
        assert!(
            tick.events.iter().any(|event| match event {
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet,
                } => match packet {
                    ClientboundPlayPacket::ContainerSetSlot(update) => update.item == cleared,
                    ClientboundPlayPacket::ContainerSetContent(update) =>
                        update.items.contains(&cleared),
                    _ => false,
                },
                _ => false,
            }),
            "target removal reaches the player's inventory"
        );
        let request = restored
            .save_request(1_786_000_001_000, 201, RegionCompression::Zlib)
            .unwrap();
        let patch = &request.player_patches[0];
        let player = Java26PlayerData::parse(
            bbb_server_persistence::created_player_document(patch, resolver).unwrap(),
            patch.uuid,
        )
        .unwrap();
        let saved = if hand == InteractionHand::MainHand {
            let Some(NbtSummaryValue::List(items)) = player.inventory_input() else {
                panic!("saved inventory list");
            };
            items
                .iter()
                .find(|value| {
                    bbb_protocol::nbt_summary::value_field(value, "Slot")
                        == Some(&NbtSummaryValue::Byte(0))
                })
                .unwrap()
        } else {
            bbb_protocol::nbt_summary::value_field(player.equipment_input().unwrap(), "offhand")
                .unwrap()
        };
        assert_eq!(
            RuntimeItemStack::read(saved, resolver).into_summary(),
            cleared
        );
    }
}

#[test]
fn compass_ticks_cover_living_equipment_and_preserve_fixed_or_other_dimension_targets() {
    use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID;
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID;
    use bbb_protocol::equipment_slots::EquipmentSlot;

    let (sender, receiver, mut world) = survival_world_holding("minecraft:compass", 1);
    let pos = BlockPos { x: 0, y: 64, z: 1 };
    place(&mut world, pos, default_state("minecraft:lodestone"));
    click(
        &sender,
        &receiver,
        &mut world,
        pos,
        InteractionHand::MainHand,
        1,
    );
    let bound = world.publication().players[0].inventory.slots[0].clone();
    let resolver = crate::world::registry_runtime::item_stack_resolver_for_test();
    let mut preserved = Vec::new();
    for (slot, tracked, dimension) in [
        (5, false, "minecraft:overworld"),
        (6, true, "minecraft:the_nether"),
    ] {
        let stack = RuntimeItemStack::from_summary(bound.clone())
            .with_component_patch(
                &DataComponentPatchSummary {
                    added_type_ids: vec![VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID],
                    lodestone_target: Some(LodestoneTargetSummary {
                        dimension: dimension.into(),
                        pos,
                    }),
                    lodestone_tracked: Some(tracked),
                    ..DataComponentPatchSummary::default()
                },
                resolver,
            )
            .unwrap()
            .into_summary();
        world.set_inventory_slot_for_test(ConnectionId(1), slot, stack.clone());
        preserved.push((slot, stack));
    }
    let dimension = Java26BuiltinDimension::Overworld;
    let (mob, _) = world.spawn_stored_entity_for_test(
        dimension,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        Vec3d {
            x: 2.5,
            y: 65.0,
            z: 2.5,
        },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID).unwrap(),
        ),
    );
    world
        .stored_entity_mut_for_test(dimension, mob)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_item_slot(
            EquipmentSlot::MainHand,
            Some(RuntimeItemStack::from_summary(bound.clone())),
        );
    world.tick(receiver.drain());
    assert_eq!(
        world
            .stored_entity_for_test(dimension, mob)
            .unwrap()
            .living()
            .unwrap()
            .item_by_slot(EquipmentSlot::MainHand)
            .unwrap()
            .summary(),
        &bound
    );
    place(&mut world, pos, default_state("minecraft:air"));
    let tick = world.tick(receiver.drain());
    let mut cleared = bound;
    cleared.component_patch.lodestone_target = None;
    assert_eq!(
        world
            .stored_entity_for_test(dimension, mob)
            .unwrap()
            .living()
            .unwrap()
            .item_by_slot(EquipmentSlot::MainHand)
            .unwrap()
            .summary(),
        &cleared
    );
    assert!(
        tick.events.iter().any(|event| matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::SetEquipment(update), .. }
                if update.entity_id == mob && update.slots.iter().any(|slot|
                    slot.slot == EquipmentSlot::MainHand && slot.item == cleared)
        )),
        "changed equipment is published to the tracking player"
    );
    for (slot, expected) in preserved {
        assert_eq!(
            world.publication().players[0].inventory.slots[slot],
            expected
        );
    }
}

#[test]
fn stacked_and_creative_compasses_split_and_full_survival_inventory_drops_output() {
    for (count, creative, full, hand, slot) in [
        (3, false, false, InteractionHand::MainHand, 0),
        (1, true, false, InteractionHand::OffHand, 40),
        (2, false, true, InteractionHand::MainHand, 0),
    ] {
        let (sender, receiver, mut world) = survival_world_holding("minecraft:compass", count);
        for index in 0..36 {
            world.set_inventory_slot_for_test(
                ConnectionId(1),
                index,
                if full {
                    ItemStackSummary {
                        item_id: Some(item_id("minecraft:stone")),
                        count: 64,
                        ..ItemStackSummary::empty()
                    }
                } else {
                    ItemStackSummary::empty()
                },
            );
        }
        world.set_inventory_slot_for_test(
            ConnectionId(1),
            slot,
            ItemStackSummary {
                item_id: Some(item_id("minecraft:compass")),
                count,
                ..ItemStackSummary::empty()
            },
        );
        if creative {
            world.set_player_mode_and_abilities_for_test(
                ConnectionId(1),
                GameType::Creative,
                PlayerAbilities::for_game_type(GameType::Creative),
            );
        }
        let at = BlockPos { x: 0, y: 64, z: 1 };
        place(&mut world, at, default_state("minecraft:lodestone"));
        click(&sender, &receiver, &mut world, at, hand, 1);
        let slots = &world.publication().players[0].inventory.slots;
        assert_eq!(slots[slot].count, if creative { count } else { count - 1 });
        assert_eq!(slots[slot].component_patch.lodestone_target, None);
        if full {
            let request = world
                .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
                .unwrap();
            let dropped = saved_entities(&request)
                .into_iter()
                .find_map(|entity| match field(&entity, "Item") {
                    value @ NbtSummaryValue::Compound(_) => Some(value.clone()),
                    _ => None,
                })
                .expect("full inventory drops bound compass");
            let encoded = dropped.clone();
            let stack = bbb_registry_codec::read_item_stack(
                &encoded,
                &bbb_registry_codec::UnavailableHolderResolver,
            )
            .unwrap();
            assert_bound(&stack, at);
        } else {
            let bound = slots
                .iter()
                .find(|stack| stack.component_patch.lodestone_target.is_some())
                .expect("new bound compass enters inventory");
            assert_bound(bound, at);
        }
    }
}
