use super::*;
use crate::entity::ai::{MemoryModuleType, MemoryValue, PackedMemory};
use crate::entity::{Entity, EntityData};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_HOGLIN_ID as HOGLIN,
    VANILLA_ENTITY_TYPE_OAK_BOAT_ID, VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID as BRUTE,
    VANILLA_ENTITY_TYPE_PIGLIN_ID as PIGLIN, VANILLA_ENTITY_TYPE_ZOGLIN_ID as ZOGLIN,
    VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID as ZOMBIFIED,
};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::{NbtSummaryEntry as Entry, NbtSummaryValue as Nbt};
use bbb_server_types::item_stack::RuntimeItemStack;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const AT: Vec3d = Vec3d {
    x: 4.5,
    y: 65.0,
    z: 0.5,
};

fn record(world: &ServerWorld, id: i32) -> Vec<Entry> {
    let Nbt::Compound(entries) = world.stored_entity_command_data_for_test(DIM, id).unwrap() else {
        unreachable!()
    };
    entries
}

fn field(world: &ServerWorld, id: i32, name: &str) -> Option<Nbt> {
    record(world, id)
        .into_iter()
        .find(|entry| entry.name == name)
        .map(|entry| entry.value)
}

fn merge(world: &mut ServerWorld, id: i32, fields: &str) {
    let Nbt::Compound(fields) = bbb_protocol::parse_snbt_compound(fields).unwrap() else {
        unreachable!()
    };
    let mut entries = record(world, id);
    for field in fields {
        entries.retain(|entry| entry.name != field.name);
        entries.push(field);
    }
    world.load_stored_entity_record_for_test(DIM, id, &entries);
}

fn spawn(world: &mut ServerWorld, kind: i32, dx: f64) -> (i32, Uuid) {
    let (id, uuid) = world.spawn_stored_entity_for_test(
        DIM,
        kind,
        Vec3d { x: AT.x + dx, ..AT },
        EntityData::GeneralLiving(GeneralLivingData::new(kind).unwrap()),
    );
    merge(
        world,
        id,
        "{NoGravity:1b,PersistenceRequired:1b,TimeInOverworld:299,CanPickUpLoot:1b}",
    );
    (id, uuid)
}

fn body(world: &ServerWorld, id: i32) -> &Entity {
    world.stored_entity_for_test(DIM, id).unwrap()
}

fn targets(world: &ServerWorld, kind: i32) -> Vec<i32> {
    world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .filter(|id| body(world, *id).entity_type_id() == kind && body(world, *id).is_alive())
        .collect()
}

fn drops(world: &ServerWorld) -> Vec<RuntimeItemStack> {
    world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .filter_map(|id| {
            let EntityData::Item(item) = body(world, id).data() else {
                return None;
            };
            Some(item.item().clone())
        })
        .collect()
}

#[test]
fn loaded_piglins_and_hoglins_cross_the_saved_conversion_threshold_with_items_and_baby_state() {
    let config = datapack_config();
    let mut candidate = restore_candidate(0, None, Vec::new());
    let mut world = spawn_pinned_restore(config.clone(), &candidate);
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let inventory = serde_json::Value::Array(
        (1..=8)
            .map(|count| {
                serde_json::json!({
                    "id":"minecraft:coal", "count":count,
                    "components":{"minecraft:custom_data":{"slot":count}}
                })
            })
            .collect(),
    )
    .to_string();
    let mut sources = Vec::new();
    for (index, kind) in [PIGLIN, BRUTE, HOGLIN].into_iter().enumerate() {
        let (id, uuid) = spawn(&mut world, kind, index as f64 * 4.0);
        if kind == HOGLIN {
            merge(&mut world, id, "{Age:-24000}");
        }
        merge(
            &mut world,
            id,
            r#"{drop_chances:{mainhand:0.75f},Health:1f,equipment:{mainhand:{id:"minecraft:diamond_sword",count:1,components:{"minecraft:damage":7}}}}"#,
        );
        if kind == PIGLIN {
            merge(&mut world, id, &format!("{{Inventory:{inventory}}}"));
        }
        let source = world.stored_entity_mut_for_test(DIM, id).unwrap();
        if kind == PIGLIN {
            source.living_mut().unwrap().set_item_slot(
                EquipmentSlot::OffHand,
                plain_stack("minecraft:gold_ingot", 1),
            );
            crate::entity::set_spawner_brain_memory(
                source,
                PackedMemory {
                    memory_type: MemoryModuleType::AdmiringItem,
                    value: MemoryValue::Bool(true),
                    time_to_live: Some(119),
                },
            );
        } else if kind == BRUTE {
            source
                .living_mut()
                .unwrap()
                .set_item_slot(EquipmentSlot::OffHand, plain_stack("minecraft:shield", 1));
        }
        sources.push((kind, id, uuid));
    }
    let sword = body(&world, sources[0].1)
        .living()
        .unwrap()
        .item_by_slot(EquipmentSlot::MainHand)
        .unwrap()
        .clone();
    assert_eq!(sword.summary().component_patch.damage, Some(7));
    let inventory_before = field(&world, sources[0].1, "Inventory").unwrap();
    world.tick(receiver.drain());
    for (_, id, _) in &sources {
        assert_eq!(field(&world, *id, "TimeInOverworld"), Some(Nbt::Int(300)));
    }
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let mut restored = spawn_pinned_restore(config, &candidate);
    restored.tick_rate_mut().set_frozen(true);
    join_loaded(&sender, &receiver, &mut restored, 1, "alice", 1);
    for (_, _, uuid) in &sources {
        let id = restored
            .live_entity_id_for_uuid_for_test(DIM, *uuid)
            .unwrap();
        assert_eq!(field(&restored, id, "TimeInOverworld"), Some(Nbt::Int(300)));
    }
    restored.tick_rate_mut().set_frozen(false);
    restored.tick(receiver.drain());
    assert_eq!(targets(&restored, ZOMBIFIED).len(), 2);
    let zoglins = targets(&restored, ZOGLIN);
    assert_eq!(zoglins.len(), 1);
    for (kind, _, uuid) in sources {
        assert!(restored
            .live_entity_id_for_uuid_for_test(DIM, uuid)
            .is_none());
        let target = if kind == HOGLIN {
            zoglins[0]
        } else {
            *targets(&restored, ZOMBIFIED)
                .iter()
                .find(|id| {
                    body(&restored, **id)
                        .living()
                        .unwrap()
                        .item_by_slot(EquipmentSlot::OffHand)
                        .is_some()
                        == (kind == BRUTE)
                })
                .unwrap()
        };
        assert_ne!(body(&restored, target).uuid(), uuid);
        let living = body(&restored, target).living().unwrap();
        assert_eq!(living.item_by_slot(EquipmentSlot::MainHand), Some(&sword));
        assert_eq!(living.health(), living.max_health());
        let nausea = living.active_effect("minecraft:nausea").unwrap();
        assert_eq!((nausea.duration(), nausea.amplifier()), (200, 0));
        let EntityData::GeneralLiving(data) = body(&restored, target).data() else {
            unreachable!()
        };
        assert_eq!(
            data.mob_retained()
                .unwrap()
                .drop_chance(EquipmentSlot::MainHand),
            0.75
        );
        assert_eq!(
            data.mob_retained().unwrap().can_pick_up_loot(),
            kind != HOGLIN
        );
    }
    let zoglin = body(&restored, zoglins[0]);
    assert!(matches!(zoglin.data(), EntityData::GeneralLiving(data) if data.is_baby()));
    assert_eq!(
        zoglin
            .living()
            .unwrap()
            .attributes()
            .value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID),
        Some(0.5)
    );
    let dropped = drops(&restored);
    let Nbt::List(inventory_before) = inventory_before else {
        unreachable!()
    };
    assert_eq!(inventory_before.len(), 8);
    let dropped_records: Vec<_> = restored
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .filter(|id| matches!(body(&restored, *id).data(), EntityData::Item(_)))
        .filter_map(|id| field(&restored, id, "Item"))
        .collect();
    for stack in inventory_before {
        assert!(
            dropped_records.contains(&stack),
            "converted piglin drops its original inventory stack"
        );
    }

    assert_eq!(
        dropped
            .iter()
            .filter(|stack| stack.item_resource_id() == Some("minecraft:gold_ingot"))
            .map(RuntimeItemStack::count)
            .sum::<i32>(),
        1
    );
    assert_eq!(
        dropped
            .iter()
            .filter(|stack| stack.item_resource_id() == Some("minecraft:coal"))
            .map(RuntimeItemStack::count)
            .sum::<i32>(),
        36
    );
    assert!(dropped.iter().all(|stack| matches!(
        stack.item_resource_id(),
        Some("minecraft:coal" | "minecraft:gold_ingot")
    )));
}

fn biome_config() -> ServerWorldConfig {
    let mut config = datapack_config();
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    for (registry, id, enabled) in [
        ("minecraft:dimension_type", "minecraft:overworld", false),
        ("minecraft:worldgen/biome", "minecraft:plains", false),
        (
            "minecraft:worldgen/biome",
            "minecraft:mushroom_fields",
            true,
        ),
    ] {
        let element = startup
            .registries
            .registries
            .iter_mut()
            .find(|r| r.registry_id == registry)
            .unwrap()
            .elements
            .iter_mut()
            .find(|element| element.id == id)
            .unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(&element.source_json).unwrap();
        value["attributes"]["minecraft:gameplay/piglins_zombify"] = serde_json::json!(enabled);
        element.source_json = serde_json::to_vec(&value).unwrap().into();
        element.source = Some(
            bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&(value)).unwrap())
                .unwrap(),
        );
    }
    config
}

#[test]
fn biome_rules_immunity_and_no_ai_control_conversion_and_live_relations_follow_the_target() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(biome_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let (id, uuid) = spawn(&mut world, HOGLIN, 0.0);
    merge(&mut world, id, "{NoAI:1b}");
    assert!(world.set_noise_biome_for_test(DIM, (1, 16, 0), "minecraft:plains"));
    world.tick(receiver.drain());
    assert_eq!(field(&world, id, "TimeInOverworld"), Some(Nbt::Int(299)));
    merge(&mut world, id, "{NoAI:0b}");
    world.tick(receiver.drain());
    assert_eq!(field(&world, id, "TimeInOverworld"), Some(Nbt::Int(0)));
    assert!(world.set_noise_biome_for_test(DIM, (1, 16, 0), "minecraft:mushroom_fields"));
    merge(
        &mut world,
        id,
        "{TimeInOverworld:299,IsImmuneToZombification:1b}",
    );
    let immune_tick = world.tick(receiver.drain());
    assert_eq!(field(&world, id, "TimeInOverworld"), Some(Nbt::Int(0)));
    assert!(immune_tick.events.iter().any(|event| matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(packet), .. }
            if packet.id == id && packet.values.iter().any(|row| row.data_id == bbb_protocol::entity_synched_data::VANILLA_HOGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID && row.value == bbb_protocol::packets::EntityDataValueKind::Boolean(true))
    )));
    merge(
        &mut world,
        id,
        "{NoAI:1b,IsImmuneToZombification:0b,TimeInOverworld:299}",
    );
    let (holder, holder_uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_COW_ID, 2.0);
    merge(&mut world, holder, "{NoAI:1b}");
    let holder_uuid_nbt = field(&world, holder, "UUID").unwrap();
    let mut entries = record(&world, id);
    entries.retain(|entry| entry.name != "leash");
    entries.push(Entry {
        name: "leash".into(),
        value: Nbt::Compound(vec![Entry {
            name: "UUID".into(),
            value: holder_uuid_nbt,
        }]),
    });
    world.load_stored_entity_record_for_test(DIM, id, &entries);
    world.tick(receiver.drain());
    assert_eq!(
        crate::entity::leash::leash_holder(body(&world, id)),
        Some(holder)
    );
    let (boat, boat_uuid) = world.spawn_stored_entity_for_test(
        DIM,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        AT,
        EntityData::Boat(crate::entity::BoatData::new()),
    );
    merge(&mut world, boat, "{NoGravity:1b}");
    console_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("ride {uuid} mount {boat_uuid}"),
    );
    console_command(&sender, &receiver, &mut world, 2, "team add converters");
    console_command(
        &sender,
        &receiver,
        &mut world,
        3,
        &format!("team join converters {uuid}"),
    );
    assert!(world.attach_external_player_for_test(ConnectionId(1), id));
    assert_eq!(world.stored_passengers_for_test(DIM, boat), vec![id]);
    assert_eq!(field(&world, id, "TimeInOverworld"), Some(Nbt::Int(299)));
    let (other, other_uuid) = spawn(&mut world, BRUTE, 0.0);
    merge(&mut world, other, "{NoAI:1b}");
    console_command(
        &sender,
        &receiver,
        &mut world,
        4,
        &format!("ride {other_uuid} mount {boat_uuid}"),
    );
    merge(&mut world, other, "{NoAI:0b,TimeInOverworld:300}");
    merge(&mut world, id, "{NoAI:0b,TimeInOverworld:300}");
    let converted = world.tick(receiver.drain());
    let target = targets(&world, ZOGLIN)[0];
    assert!(world.live_entity_id_for_uuid_for_test(DIM, uuid).is_none());
    let unleased_target = targets(&world, ZOMBIFIED)[0];
    assert_eq!(
        world.stored_passengers_for_test(DIM, boat),
        vec![unleased_target]
    );
    assert_eq!(
        world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .2,
        Some(target)
    );
    assert_eq!(
        crate::entity::leash::leash_holder(body(&world, target)),
        Some(holder)
    );
    assert_eq!(body(&world, holder).uuid(), holder_uuid);
    assert_eq!(
        world
            .scoreboard()
            .players_team(&body(&world, target).uuid().to_string())
            .map(|team| team.name.try_to_string().unwrap())
            .as_deref(),
        Some("converters")
    );
    assert!(world.scoreboard().players_team(&uuid.to_string()).is_none());
    let player = world.player_entity_id_for_test(ConnectionId(1));
    let mut events = converted.events;
    let has_passengers = |events: &[WorldEvent]| {
        events.iter().any(|event| {
            matches!(event,
                WorldEvent::Send { packet: ClientboundPlayPacket::SetPassengers(packet), .. }
                if packet.vehicle_id == target && packet.passenger_ids == vec![player]
            )
        })
    };
    for _ in 0..8 {
        if has_passengers(&events) {
            break;
        }
        events.extend(
            play(
                &sender,
                &receiver,
                &mut world,
                1,
                PlayServerbound::ChunkBatchReceived {
                    desired_chunks_per_tick: 64.0,
                },
            )
            .events,
        );
    }
    assert!(
        has_passengers(&events),
        "converted mount publishes passengers to its rider; viewers={:?}",
        world.player_tracking_viewers_for_test(DIM, target)
    );
    let spawn = events.iter().position(|event| matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::AddEntity(packet), .. } if packet.id == target
    )).expect("the rider receives the converted mount");
    let passengers = events
        .iter()
        .position(|event| {
            matches!(event,
                WorldEvent::Send { packet: ClientboundPlayPacket::SetPassengers(packet), .. }
                if packet.vehicle_id == target && packet.passenger_ids == vec![player]
            )
        })
        .unwrap();
    assert!(spawn < passengers);
}
