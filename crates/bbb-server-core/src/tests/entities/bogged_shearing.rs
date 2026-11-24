use super::horse_interaction::{hold, item_id, record};
use super::*;
use bbb_protocol::{NbtSummaryEntry as Entry, NbtSummaryValue as Nbt};

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const BOGGED: i32 = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BOGGED_ID;

fn spawn(world: &mut ServerWorld, position: Vec3d, fields: &str) -> i32 {
    let id = world
        .spawn_stored_entity_for_test(
            DIM,
            BOGGED,
            position,
            crate::entity::EntityData::GeneralLiving(GeneralLivingData::new(BOGGED).unwrap()),
        )
        .0;
    let Nbt::Compound(fields) = bbb_protocol::parse_snbt_compound(fields).unwrap() else {
        unreachable!()
    };
    let mut entries = record(world, id);
    for field in fields.into_iter().chain([Entry {
        name: "NoAI".into(),
        value: Nbt::Byte(1),
    }]) {
        entries.retain(|entry| entry.name != field.name);
        entries.push(field);
    }
    world.load_stored_entity_record_for_test(DIM, id, &entries);
    id
}

fn drops(world: &ServerWorld) -> Vec<(i32, i32)> {
    world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .filter_map(|id| {
            let crate::entity::EntityData::Item(data) =
                world.stored_entity_for_test(DIM, id)?.data()
            else {
                return None;
            };
            Some((data.item().item_id()?, data.item().count()))
        })
        .collect()
}

fn sheared(world: &ServerWorld, id: i32) -> bool {
    record(world, id)
        .iter()
        .any(|field| field.name == "sheared" && field.value == Nbt::Byte(1))
}

fn interact(id: i32, hand: InteractionHand) -> PlayServerbound {
    PlayServerbound::Interact(InteractEntity {
        entity_id: id,
        hand,
        location: Vec3d::default(),
        using_secondary_action: false,
    })
}

#[test]
fn live_bogged_shearing_syncs_drops_both_hands_durability_and_saved_state() {
    for (game_type, hand, slot) in [
        (GameType::Survival, InteractionHand::MainHand, 0),
        (GameType::Creative, InteractionHand::OffHand, 40),
    ] {
        let (sender, receiver) = mailbox();
        let mut config = datapack_config();
        config.game_type = game_type;
        let mut world = spawn_pinned_world(config.clone());
        join_loaded(&sender, &receiver, &mut world, 1, "shearer", 0x50ee);
        world.tick_rate_mut().set_frozen(true);
        let position = Vec3d {
            z: world.publication().players[0].position.z + 1.0,
            ..world.publication().players[0].position
        };
        let id = spawn(&mut world, position, "{Silent:1b}");
        let uuid = world.stored_entity_for_test(DIM, id).unwrap().uuid();
        world.set_inventory_slot_for_test(
            ConnectionId(1),
            slot,
            ItemStackSummary {
                item_id: Some(item_id("minecraft:shears")),
                count: 1,
                component_patch: Default::default(),
            },
        );
        let tick = play(&sender, &receiver, &mut world, 1, interact(id, hand));
        assert!(sheared(&world, id));
        let mushrooms = drops(&world);
        assert_eq!(mushrooms.iter().map(|(_, count)| count).sum::<i32>(), 2);
        assert!(mushrooms.iter().all(|(id, _)| [
            item_id("minecraft:brown_mushroom"),
            item_id("minecraft:red_mushroom")
        ]
        .contains(id)));
        let damage = if game_type == GameType::Survival {
            Some(1)
        } else {
            None
        };
        assert_eq!(
            tick.snapshot.players[0].inventory.slots[slot]
                .component_patch
                .damage,
            damage
        );
        assert!(tick.events.iter().any(|event| matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(data), .. }
            if data.id == id && data.values.iter().any(|row| row.data_id == bbb_protocol::entity_synched_data::VANILLA_BOGGED_SHEARED_DATA_ID && row.value == bbb_protocol::packets::EntityDataValueKind::Boolean(true))
        )));
        assert!(tick.events.iter().any(|event| matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. }
            if sound.source == SoundSource::Players && sound.volume == 1.0 && sound.pitch == 1.0 && matches!(sound.sound, SoundEventHolder::Reference { registry_id } if vanilla_sound_event_resource_id(registry_id) == Some("minecraft:entity.bogged.shear"))
        )));
        for event in [VanillaGameEvent::SHEAR, VanillaGameEvent::ENTITY_INTERACT] {
            assert!(world
                .game_events(DIM)
                .0
                .iter()
                .any(|value| value.game_event == event
                    && value.source_entity_id
                        == Some(world.player_entity_id_for_test(ConnectionId(1)))));
        }
        let again = play(&sender, &receiver, &mut world, 1, interact(id, hand));
        assert_eq!(drops(&world), mushrooms);
        assert_eq!(
            again.snapshot.players[0].inventory.slots[slot]
                .component_patch
                .damage,
            damage
        );
        let request = world
            .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
            .unwrap();
        let mut candidate = restore_candidate(10, None, Vec::new());
        restore_entity_writes(&mut candidate, &request.entity_writes);
        let mut restored = spawn_pinned_restore(config, &candidate);
        restored.tick_rate_mut().set_frozen(true);
        let id = restored
            .live_entity_id_for_uuid_for_test(DIM, uuid)
            .unwrap();
        assert!(sheared(&restored, id));
        let (joined, loaded) =
            join_loaded_ticks(&sender, &receiver, &mut restored, 1, "shearer", 0x50ee);
        let has_metadata = |tick: &TickResult| {
            tick.events.iter().any(|event| matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(data), .. }
            if data.id == id && data.values.iter().any(|row| row.data_id == bbb_protocol::entity_synched_data::VANILLA_BOGGED_SHEARED_DATA_ID && row.value == bbb_protocol::packets::EntityDataValueKind::Boolean(true))
        ))
        };
        let mut synced = has_metadata(&joined) || has_metadata(&loaded);
        for _ in 0..16 {
            if synced {
                break;
            }
            let tick = play(
                &sender,
                &receiver,
                &mut restored,
                1,
                PlayServerbound::ChunkBatchReceived {
                    desired_chunks_per_tick: 64.0,
                },
            );
            synced |= has_metadata(&tick);
        }
        if !synced {
            let position = restored.publication().players[0].position;
            let tick = play(
                &sender,
                &receiver,
                &mut restored,
                1,
                PlayServerbound::MovePlayer(bbb_protocol::packets::MovePlayer {
                    position: Some(position),
                    rotation: None,
                    on_ground: true,
                    horizontal_collision: false,
                }),
            );
            synced = has_metadata(&tick);
        }
        assert!(synced);
        hold(&mut restored, "minecraft:shears", 1);
        let original = drops(&restored);
        let tick = play(
            &sender,
            &receiver,
            &mut restored,
            1,
            interact(id, InteractionHand::MainHand),
        );
        assert_eq!(drops(&restored), original);
        assert_eq!(
            tick.snapshot.players[0].inventory.slots[0]
                .component_patch
                .damage,
            None
        );
    }
}

#[test]
fn live_bogged_shearing_uses_dynamic_tool_and_location_loot_without_splitting_stacks() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    let registry = startup
        .loot
        .registries
        .iter_mut()
        .find(|value| value.registry_id == "minecraft:loot_table")
        .unwrap();
    let element = registry
        .elements
        .iter_mut()
        .find(|value| value.id == "minecraft:shearing/bogged")
        .unwrap();
    element.value = bbb_server_datapack::loot::LootElementValue::Table(bbb_server_datapack::loot::parse_loot_table(&serde_json::json!({
        "type":"minecraft:shearing", "pools":[{"rolls":1,"conditions":[
            {"condition":"minecraft:match_tool","predicate":{"items":"minecraft:shears"}},
            {"condition":"minecraft:location_check","predicate":{"block":{"blocks":"minecraft:air"}}},
            {"condition":"minecraft:entity_properties","entity":"this","predicate":{"type":"minecraft:bogged"}}
        ],"entries":[{"type":"minecraft:item","name":"minecraft:diamond","functions":[{"function":"minecraft:set_count","count":3}]}]}]
    }), None).unwrap());
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "shearer", 0x50ee);
    world.tick_rate_mut().set_frozen(true);
    let position = Vec3d {
        z: world.publication().players[0].position.z + 1.0,
        ..world.publication().players[0].position
    };
    let id = spawn(&mut world, position, "{}");
    hold(&mut world, "minecraft:shears", 1);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact(id, InteractionHand::MainHand),
    );
    assert_eq!(drops(&world), vec![(item_id("minecraft:diamond"), 3)]);
    assert!(sheared(&world, id));
}

#[test]
fn dispenser_bogged_shearing_reuses_shear_and_refuses_sheared_or_dead_targets() {
    for (fields, expected) in [
        ("{Silent:1b}", true),
        ("{sheared:1b}", false),
        ("{Health:0.0f}", false),
    ] {
        let (sender, receiver) = mailbox();
        let mut world = spawn_pinned_world(datapack_config());
        join_loaded(&sender, &receiver, &mut world, 1, "viewer", 0x50ef);
        console_command(&sender, &receiver, &mut world, 1, "forceload add 0 0");
        console_command(
            &sender,
            &receiver,
            &mut world,
            2,
            "setblock 0 63 1 minecraft:stone",
        );
        let id = spawn(
            &mut world,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 1.5,
            },
            fields,
        );
        console_command(
            &sender,
            &receiver,
            &mut world,
            3,
            r#"setblock 0 64 0 minecraft:dispenser[facing=south]{Items:[{Slot:0b,id:"minecraft:shears",count:1}]}"#,
        );
        console_command(
            &sender,
            &receiver,
            &mut world,
            4,
            "setblock 1 64 0 minecraft:redstone_block",
        );
        world.tick_rate_mut().set_frozen(false);
        let mut sound = false;
        for _ in 0..5 {
            let tick = world.tick(receiver.drain());
            sound |= tick.events.iter().any(|event| matches!(event,
                WorldEvent::Send {packet:ClientboundPlayPacket::Sound(sound),..}
                if sound.source == SoundSource::Blocks && matches!(sound.sound, SoundEventHolder::Reference {registry_id} if vanilla_sound_event_resource_id(registry_id) == Some("minecraft:entity.bogged.shear"))
            ));
        }
        assert_eq!(sound, expected);
        assert_eq!(
            drops(&world).iter().map(|(_, count)| count).sum::<i32>(),
            if expected { 2 } else { 0 }
        );
        if expected {
            assert!(sheared(&world, id));
        }
        console_command(
            &sender,
            &receiver,
            &mut world,
            5,
            "setblock 1 64 0 minecraft:air",
        );
        console_command(
            &sender,
            &receiver,
            &mut world,
            6,
            "setblock 1 64 0 minecraft:redstone_block",
        );
        for _ in 0..5 {
            world.tick(receiver.drain());
        }
        assert_eq!(
            drops(&world).iter().map(|(_, count)| count).sum::<i32>(),
            if expected { 2 } else { 0 }
        );
        let shears = world.with_level_for_test(DIM, |level| {
            let container = level
                .block_container(BlockPos { x: 0, y: 64, z: 0 }, true)
                .unwrap();
            level.get_container_item(container, 0).unwrap().unwrap()
        });
        assert_eq!(
            shears.summary().component_patch.damage,
            if expected { Some(1) } else { None }
        );
    }
}
