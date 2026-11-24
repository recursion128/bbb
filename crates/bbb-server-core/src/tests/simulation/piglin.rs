use super::*;

use crate::entity::ai::{MemoryModuleType, MemoryValue, PackedMemory};
use crate::entity::{EntityData, GeneralLivingData};
use crate::tests::entities::restore_entity_writes;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID, VANILLA_ENTITY_TYPE_PIGLIN_ID,
    VANILLA_ENTITY_TYPE_WITHER_ID, VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID,
};
use bbb_protocol::packets::InteractEntity;
use bbb_server_types::item_stack::RuntimeItemStack;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const AT: Vec3d = Vec3d {
    x: 2.5,
    y: 65.0,
    z: 0.5,
};

fn config(game_type: GameType) -> ServerWorldConfig {
    let mut config = datapack_config();
    config.game_type = game_type;
    config.force_game_mode = true;
    config.keep_alive_interval_ticks = 10_000;
    config.admin_lists = operator_config(&[("alice", 1)]).admin_lists;
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    startup
        .tags
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:item")
        .unwrap()
        .tags
        .get_mut("minecraft:piglin_loved")
        .unwrap()
        .push("minecraft:clay_ball".into());
    let table = startup
        .loot
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:loot_table")
        .unwrap()
        .elements
        .iter_mut()
        .find(|element| element.id == "minecraft:gameplay/piglin_bartering")
        .unwrap();
    table.value = bbb_server_datapack::loot::LootElementValue::Table(
        bbb_server_datapack::loot::parse_loot_table(&serde_json::json!({
            "type":"minecraft:barter", "pools":[{"rolls":1,
                "conditions":[{"condition":"minecraft:entity_properties","entity":"this","predicate":{"type":"minecraft:piglin"}}],
                "entries":[{"type":"minecraft:item","name":"minecraft:diamond", "functions":[{"function":"minecraft:set_components","components":{"minecraft:max_stack_size":7}}]}]
            }]
        }), None).unwrap());
    config
}

fn fixture(
    game_type: GameType,
) -> (
    WorldCommandSender,
    WorldCommandReceiver,
    ServerWorld,
    Java26WorldCandidate,
) {
    fixture_with_config(config(game_type))
}

fn fixture_with_config(
    config: ServerWorldConfig,
) -> (
    WorldCommandSender,
    WorldCommandReceiver,
    ServerWorld,
    Java26WorldCandidate,
) {
    let candidate = restore_candidate(0, None, Vec::new());
    let (sender, receiver, world) = helpers::restored_world(config, &candidate);
    (sender, receiver, world, candidate)
}

fn spawn(world: &mut ServerWorld, baby: bool) -> (i32, Uuid) {
    let (id, uuid) = world.spawn_stored_entity_for_test(
        DIM,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        AT,
        EntityData::GeneralLiving(GeneralLivingData::new(VANILLA_ENTITY_TYPE_PIGLIN_ID).unwrap()),
    );
    let mut record = match world.stored_entity_command_data_for_test(DIM, id).unwrap() {
        NbtSummaryValue::Compound(record) => record,
        _ => unreachable!(),
    };
    record.retain(|entry| entry.name != "CanPickUpLoot");
    for (name, value) in [
        ("IsBaby", NbtSummaryValue::Byte(i8::from(baby))),
        ("NoGravity", NbtSummaryValue::Byte(1)),
        ("IsImmuneToZombification", NbtSummaryValue::Byte(1)),
        ("PersistenceRequired", NbtSummaryValue::Byte(1)),
    ] {
        record.retain(|entry| entry.name != name);
        record.push(nbt(name, value));
    }
    world.load_stored_entity_record_for_test(DIM, id, &record);
    world
        .stored_entity_mut_for_test(DIM, id)
        .unwrap()
        .set_pos(AT);
    (id, uuid)
}

fn held(world: &ServerWorld, id: i32, slot: EquipmentSlot) -> Option<RuntimeItemStack> {
    world
        .stored_entity_for_test(DIM, id)?
        .living()?
        .item_by_slot(slot)
        .cloned()
}

fn persistent(world: &ServerWorld, id: i32) -> bool {
    matches!(world.stored_entity_for_test(DIM, id).unwrap().data(),
        EntityData::GeneralLiving(data) if data.mob.as_ref().unwrap().persistence_required())
}

fn interact(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    id: i32,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::Interact(InteractEntity {
            entity_id: id,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    )
}

fn diamonds(world: &ServerWorld) -> Vec<i32> {
    world.stored_entity_ids_for_test(DIM).into_iter().filter(|id| matches!(world.stored_entity_for_test(DIM,*id).map(|entity| entity.data()),Some(EntityData::Item(item)) if item.item().item_resource_id()==Some("minecraft:diamond"))).collect()
}

fn patched_gold() -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id("minecraft:gold_ingot")),
        count: 3,
        component_patch: DataComponentPatchSummary {
            added_type_ids: vec![
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
            ],
            custom_data: Some(NbtSummaryValue::Compound(vec![nbt(
                "kept",
                NbtSummaryValue::Int(7),
            )])),
            ..Default::default()
        }
        .into(),
    }
}

#[test]
fn packet_accepts_one_actual_stack_refuses_while_admiring_and_draws_the_barter_table() {
    for mode in [GameType::Survival, GameType::Creative] {
        let (sender, receiver, mut world, _) = fixture(mode);
        let (id, uuid) = spawn(&mut world, false);
        let old = crate::entity::item::plain_stack("minecraft:stick", 2).unwrap();
        world
            .stored_entity_mut_for_test(DIM, id)
            .unwrap()
            .living_mut()
            .unwrap()
            .set_item_slot(EquipmentSlot::OffHand, Some(old));
        let gold = patched_gold();
        world.stock_menu_slot_for_test(ConnectionId(1), 36, gold.clone());
        interact(&sender, &receiver, &mut world, id);
        let offhand = held(&world, id, EquipmentSlot::OffHand).unwrap();
        assert_eq!(offhand.count(), 1);
        assert_eq!(
            offhand.summary().component_patch.custom_data,
            gold.component_patch.custom_data
        );
        let count = if mode == GameType::Creative { 3 } else { 2 };
        assert_eq!(
            world
                .player_inventory_slot_for_test(ConnectionId(1), 0)
                .count,
            count
        );
        interact(&sender, &receiver, &mut world, id);
        assert_eq!(
            world
                .player_inventory_slot_for_test(ConnectionId(1), 0)
                .count,
            count
        );
        assert!(world
            .stored_items_for_test(DIM)
            .iter()
            .any(|item| item.2 == "minecraft:stick" && item.3 == 2));
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("data merge entity {uuid} {{NoAI:1b}}"),
        );
        for _ in 0..125 {
            world.tick(receiver.drain());
        }
        assert!(diamonds(&world).is_empty());
        assert!(held(&world, id, EquipmentSlot::OffHand).is_some());
        interact(&sender, &receiver, &mut world, id);
        assert_eq!(
            world
                .player_inventory_slot_for_test(ConnectionId(1), 0)
                .count,
            count
        );
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("data merge entity {uuid} {{NoAI:0b}}"),
        );
        for _ in 0..125 {
            world.tick(receiver.drain());
        }
        let drops = diamonds(&world);
        assert_eq!(drops.len(), 1);
        let item = world.stored_entity_for_test(DIM, drops[0]).unwrap();
        let EntityData::Item(data) = item.data() else {
            unreachable!()
        };
        assert_eq!(data.item().max_stack_size().unwrap(), 7);
        assert_eq!(
            data.thrower(),
            Some(world.stored_entity_for_test(DIM, id).unwrap().uuid())
        );
        assert!(held(&world, id, EquipmentSlot::OffHand).is_none());
    }

    use crate::tests::entities::{console_command, console_command_answer};

    let cargo = serde_json::json!({"type":"minecraft:chest","pools":[{
        "rolls":1,"conditions":[{"condition":"minecraft:inverted","term":{
            "condition":"minecraft:entity_properties","entity":"this","predicate":{}
        }}],"entries":[{"type":"minecraft:item","name":"minecraft:diamond"}]
    }]});
    let vehicle_condition = |item| {
        serde_json::json!({
            "condition":"minecraft:entity_properties","entity":"this",
            "predicate":{"type":"minecraft:piglin","vehicle":{
                "type":"minecraft:oak_chest_boat",
                "slots":{"container.*":{"items":item,"count":1}}
            }}
        })
    };
    let barter = serde_json::json!({"type":"minecraft:barter","pools":[
        {"rolls":1,"entries":[{"type":"minecraft:item","name":"minecraft:apple"}]},
        {"rolls":1,"conditions":[vehicle_condition("minecraft:diamond")],
            "entries":[{"type":"minecraft:item","name":"minecraft:diamond"}]},
        {"rolls":1,"conditions":[vehicle_condition("minecraft:stone")],
            "entries":[{"type":"minecraft:item","name":"minecraft:emerald"}]}
    ]});
    let root = std::env::temp_dir().join(format!("bbb-barter-vehicle-slots-{}", Uuid::new_v4()));
    let pack = root.join("cargo");
    std::fs::create_dir_all(pack.join("data/test/loot_table")).unwrap();
    std::fs::create_dir_all(pack.join("data/minecraft/loot_table/gameplay")).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"Barter vehicle slots","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
    for (path, table) in [
        ("data/test/loot_table/pending_vehicle.json", cargo),
        (
            "data/minecraft/loot_table/gameplay/piglin_bartering.json",
            barter,
        ),
    ] {
        std::fs::write(pack.join(path), serde_json::to_vec(&table).unwrap()).unwrap();
    }
    let startup = bbb_server_datapack::startup::DatapackStartup::load(
        &bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: Some(root.clone()),
            saved_data_packs: None,
            safe_mode: false,
            limits: Default::default(),
        },
    )
    .unwrap();
    let config = ServerWorldConfig {
        datapacks: Some(Arc::new(startup)),
        game_type: GameType::Survival,
        force_game_mode: true,
        ..operator_config(&[("alice", 1)])
    };
    std::fs::remove_dir_all(root).unwrap();
    let (sender, receiver, mut world, _) = fixture_with_config(config.clone());
    for (index, command) in [
        "item replace entity alice armor.head with minecraft:golden_helmet",
        "summon minecraft:oak_chest_boat 2.5 65 0.5 {UUID:[I;0,0,0,4001],NoGravity:1b,LootTable:'test:pending_vehicle',LootTableSeed:37L}",
        "summon minecraft:piglin 5.5 65 0.5 {UUID:[I;0,0,0,4002],NoGravity:1b,IsImmuneToZombification:1b,PersistenceRequired:1b}",
        "ride 00000000-0000-0000-0000-000000000fa2 mount 00000000-0000-0000-0000-000000000fa1",
        "item replace entity alice weapon.mainhand with minecraft:gold_ingot 2",
    ].into_iter().enumerate() {
        let tick = console_command(&sender, &receiver, &mut world, index as u64 + 1, command);
        assert_eq!(console_command_answer(&tick).0, Some(1), "{command}");
    }
    let boat_uuid = Uuid::from_u128(4001);
    let piglin_uuid = Uuid::from_u128(4002);
    let boat = world
        .live_entity_id_for_uuid_for_test(DIM, boat_uuid)
        .unwrap();
    let piglin = world
        .live_entity_id_for_uuid_for_test(DIM, piglin_uuid)
        .unwrap();
    assert_eq!(world.stored_passengers_for_test(DIM, boat), &[piglin]);
    let body = world.stored_entity_for_test(DIM, piglin).unwrap();
    assert_eq!(body.uuid(), piglin_uuid);
    assert_eq!(body.entity_type_id(), VANILLA_ENTITY_TYPE_PIGLIN_ID);
    assert!(body.is_alive_with_health());
    assert!(
        matches!(body.data(), EntityData::GeneralLiving(data) if !data.is_no_ai() && !data.is_baby())
    );
    let boat_body = world.stored_entity_for_test(DIM, boat).unwrap();
    assert!(crate::entity::item_slots::container_has_loot(boat_body));
    for slot in 0..crate::entity::item_slots::container_size(boat_body).unwrap() {
        assert!(crate::entity::item_slots::read(boat_body, slot as i32)
            .unwrap()
            .is_some_and(|stack| stack.is_empty()));
    }
    let NbtSummaryValue::Compound(before) = world
        .stored_entity_command_data_for_test(DIM, boat)
        .unwrap()
    else {
        panic!("the pending boat has canonical entity data");
    };
    assert!(before.iter().any(|entry| entry.name == "LootTable"
        && entry.value == NbtSummaryValue::String("test:pending_vehicle".into())));
    assert!(before
        .iter()
        .any(|entry| entry.name == "LootTableSeed" && entry.value == NbtSummaryValue::Long(37)));
    assert!(!before.iter().any(|entry| entry.name == "Items"));
    assert!(world.stored_items_for_test(DIM).is_empty());
    interact(&sender, &receiver, &mut world, piglin);
    let offhand = held(&world, piglin, EquipmentSlot::OffHand).unwrap();
    assert_eq!(offhand.item_resource_id(), Some("minecraft:gold_ingot"));
    assert_eq!(offhand.count(), 1);
    assert_eq!(
        world
            .player_inventory_slot_for_test(ConnectionId(1), 0)
            .count,
        1
    );
    let tick = console_command(
        &sender,
        &receiver,
        &mut world,
        6,
        "gamemode spectator alice",
    );
    assert_eq!(console_command_answer(&tick).0, Some(1));
    for _ in 0..250 {
        world.tick(receiver.drain());
        if held(&world, piglin, EquipmentSlot::OffHand).is_none() {
            break;
        }
    }
    assert!(held(&world, piglin, EquipmentSlot::OffHand).is_none());
    let mut drops: Vec<_> = world
        .stored_items_for_test(DIM)
        .into_iter()
        .map(|item| (item.2, item.3))
        .collect();
    drops.sort();
    assert_eq!(
        drops,
        vec![
            ("minecraft:apple".into(), 1),
            ("minecraft:diamond".into(), 1)
        ]
    );
    for (dimension, chunk) in world.pending_runtime_entity_chunks() {
        world
            .complete_runtime_entity_chunk(&dimension, chunk, None)
            .unwrap();
    }
    let saved = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    let mut candidate = restore_candidate(world.game_time(), None, Vec::new());
    restore_entity_writes(&mut candidate, &saved.entity_writes);
    let restored = spawn_pinned_restore(config, &candidate);
    for current in [&world, &restored] {
        let boat = current
            .live_entity_id_for_uuid_for_test(DIM, boat_uuid)
            .unwrap();
        let piglin = current
            .live_entity_id_for_uuid_for_test(DIM, piglin_uuid)
            .unwrap();
        assert_eq!(current.stored_passengers_for_test(DIM, boat), &[piglin]);
        let piglin_body = current.stored_entity_for_test(DIM, piglin).unwrap();
        assert_eq!(piglin_body.uuid(), piglin_uuid);
        assert!(piglin_body.is_alive_with_health());
        assert!(matches!(piglin_body.data(), EntityData::GeneralLiving(data) if !data.is_no_ai()));
        let boat_body = current.stored_entity_for_test(DIM, boat).unwrap();
        assert_eq!(boat_body.uuid(), boat_uuid);
        assert_eq!(boat_body.removal_reason(), None);
        assert!(!crate::entity::item_slots::container_has_loot(boat_body));
        let cargo: Vec<_> = (0..crate::entity::item_slots::container_size(boat_body).unwrap())
            .filter_map(|slot| crate::entity::item_slots::read(boat_body, slot as i32).unwrap())
            .filter(|stack| !stack.is_empty())
            .map(|stack| (stack.item_resource_id().unwrap().to_owned(), stack.count()))
            .collect();
        assert_eq!(cargo, vec![("minecraft:diamond".into(), 1)]);
        let NbtSummaryValue::Compound(record) = current
            .stored_entity_command_data_for_test(DIM, boat)
            .unwrap()
        else {
            panic!("the unpacked boat has canonical entity data");
        };
        assert!(!record.iter().any(|entry| entry.name == "LootTable"));
        let Some(NbtSummaryValue::List(items)) = record
            .iter()
            .find(|entry| entry.name == "Items")
            .map(|entry| &entry.value)
        else {
            panic!("the unpacked boat writes its cargo");
        };
        let [NbtSummaryValue::Compound(stack)] = items.as_slice() else {
            panic!("the boat saves exactly one stack: {items:?}");
        };
        assert!(stack.iter().any(|entry| entry.name == "id"
            && entry.value == NbtSummaryValue::String("minecraft:diamond".into())));
        assert!(stack
            .iter()
            .any(|entry| entry.name == "count" && entry.value == NbtSummaryValue::Int(1)));
    }
}

#[test]
fn ground_pickup_takes_one_preserves_the_remainder_and_babies_do_not_barter() {
    for (baby, item) in [
        (false, "minecraft:gold_ingot"),
        (true, "minecraft:gold_ingot"),
        (false, "minecraft:raw_gold"),
        (true, "minecraft:raw_gold"),
        (false, "minecraft:clay_ball"),
    ] {
        let (_, receiver, mut world, mut candidate) = fixture(GameType::Creative);
        let (mut id, uuid) = spawn(&mut world, baby);
        if item != "minecraft:gold_ingot" {
            let NbtSummaryValue::Compound(mut record) =
                world.stored_entity_command_data_for_test(DIM, id).unwrap()
            else {
                unreachable!()
            };
            record.retain(|entry| entry.name != "PersistenceRequired");
            record.push(nbt("PersistenceRequired", NbtSummaryValue::Byte(0)));
            world.load_stored_entity_record_for_test(DIM, id, &record);
            assert!(!persistent(&world, id));
        }
        if baby || item != "minecraft:gold_ingot" {
            world
                .stored_entity_mut_for_test(DIM, id)
                .unwrap()
                .living_mut()
                .unwrap()
                .set_item_slot(
                    EquipmentSlot::MainHand,
                    Some(crate::entity::item::plain_stack("minecraft:golden_sword", 1).unwrap()),
                );
        }
        let mut gold = patched_gold();
        gold.item_id = Some(item_id(item));
        let ground = world.spawn_ground_item_for_test(DIM, AT, &gold);
        let tick = world.tick(receiver.drain());
        assert!(tick.events.iter().any(|event|matches!(event,WorldEvent::Send {packet:ClientboundPlayPacket::TakeItemEntity(take),..} if take.item_id==ground && take.player_id==id && take.amount==1)));
        if item != "minecraft:gold_ingot" {
            assert!(persistent(&world, id));
        }
        let remainder = world.stored_entity_for_test(DIM, ground).unwrap();
        let EntityData::Item(data) = remainder.data() else {
            unreachable!()
        };
        assert_eq!(data.item().count(), 2);
        assert_eq!(
            data.item().summary().component_patch.custom_data,
            gold.component_patch.custom_data
        );
        world
            .stored_entity_mut_for_test(DIM, ground)
            .unwrap()
            .set_pos(Vec3d { x: 10.5, ..AT });
        assert_eq!(
            held(&world, id, EquipmentSlot::OffHand)
                .unwrap()
                .summary()
                .component_patch
                .custom_data,
            gold.component_patch.custom_data
        );
        if !baby && item != "minecraft:gold_ingot" {
            for _ in 0..35 {
                world.tick(receiver.drain());
            }
            for (dimension, chunk) in world.pending_runtime_entity_chunks() {
                world
                    .complete_runtime_entity_chunk(&dimension, chunk, None)
                    .unwrap();
            }
            let request = world
                .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
                .unwrap();
            restore_entity_writes(&mut candidate, &request.entity_writes);
            world = spawn_pinned_restore(config(GameType::Creative), &candidate);
            id = world.live_entity_id_for_uuid_for_test(DIM, uuid).unwrap();
            assert!(persistent(&world, id));
            let (sender, join_receiver) = mailbox();
            join_loaded(&sender, &join_receiver, &mut world, 1, "alice", 1);
            for _ in 0..50 {
                world.tick(receiver.drain());
            }
            assert!(held(&world, id, EquipmentSlot::OffHand).is_some());
        }
        for _ in 0..125 {
            world.tick(receiver.drain());
        }
        if baby {
            assert_eq!(
                held(&world, id, EquipmentSlot::MainHand)
                    .unwrap()
                    .item_resource_id(),
                Some(item)
            );
            assert!(diamonds(&world).is_empty());
            let NbtSummaryValue::Compound(record) =
                world.stored_entity_command_data_for_test(DIM, id).unwrap()
            else {
                unreachable!()
            };
            assert!(record.iter().any(|entry| entry.name == "Inventory" && matches!(&entry.value,
                NbtSummaryValue::List(items) if items.iter().any(|item| matches!(item,
                    NbtSummaryValue::Compound(fields) if fields.iter().any(|field| field.name == "id" && field.value == NbtSummaryValue::String("minecraft:golden_sword".into())))))));
        } else if item == "minecraft:gold_ingot" {
            assert_eq!(diamonds(&world).len(), 1);
        } else {
            assert!(diamonds(&world).is_empty());
            assert!(held(&world, id, EquipmentSlot::OffHand).is_none());
            let EntityData::GeneralLiving(data) =
                world.stored_entity_for_test(DIM, id).unwrap().data()
            else {
                unreachable!()
            };
            let stored = data
                .internal_inventory()
                .unwrap()
                .original_items()
                .find(|stack| stack.item_resource_id() == Some(item))
                .unwrap();
            assert_eq!(stored.count(), 1);
            assert_eq!(
                stored.summary().component_patch.custom_data,
                gold.component_patch.custom_data
            );
        }
    }
}

#[test]
fn saved_admiration_restores_remaining_time_and_barters_once_and_player_hurt_disables_it() {
    let (sender, receiver, mut world, mut candidate) = fixture(GameType::Creative);
    let (id, uuid) = spawn(&mut world, false);
    world.stock_menu_slot_for_test(ConnectionId(1), 36, patched_gold());
    interact(&sender, &receiver, &mut world, id);
    let other = PackedMemory {
        memory_type: MemoryModuleType::HuntedRecently,
        value: MemoryValue::Bool(true),
        time_to_live: Some(500),
    };
    crate::entity::set_spawner_brain_memory(
        world.stored_entity_mut_for_test(DIM, id).unwrap(),
        other.clone(),
    );
    for _ in 0..35 {
        world.tick(receiver.drain());
    }
    let remaining = crate::entity::brain_packed(world.stored_entity_for_test(DIM, id).unwrap())
        .unwrap()
        .memories
        .iter()
        .find(|memory| memory.memory_type == MemoryModuleType::AdmiringItem)
        .unwrap()
        .time_to_live
        .unwrap();
    assert!(remaining > 0 && remaining < 119);
    let other = crate::entity::brain_packed(world.stored_entity_for_test(DIM, id).unwrap())
        .unwrap()
        .memories
        .iter()
        .find(|memory| memory.memory_type == MemoryModuleType::HuntedRecently)
        .unwrap()
        .clone();
    assert!(other.time_to_live.is_some_and(|ttl| ttl > 0 && ttl < 500));
    for (dimension, chunk) in world.pending_runtime_entity_chunks() {
        world
            .complete_runtime_entity_chunk(&dimension, chunk, None)
            .unwrap();
    }
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let mut restored = spawn_pinned_restore(config(GameType::Creative), &candidate);
    let restored_id = restored
        .live_entity_id_for_uuid_for_test(DIM, uuid)
        .unwrap();
    assert!(held(&restored, restored_id, EquipmentSlot::OffHand).is_some());
    assert!(crate::entity::brain_packed(
        restored.stored_entity_for_test(DIM, restored_id).unwrap()
    )
    .unwrap()
    .memories
    .contains(&other));
    let (sender2, receiver2) = mailbox();
    join_loaded(&sender2, &receiver2, &mut restored, 1, "alice", 1);
    chat_command(&sender2, &receiver2, &mut restored, 1, "tp alice 10 65 10");
    for _ in 0..250 {
        restored.tick(receiver2.drain());
    }
    assert_eq!(diamonds(&restored).len(), 1);
    chat_command(
        &sender2,
        &receiver2,
        &mut restored,
        1,
        "tp alice 0.5 65 0.5",
    );
    restored.stock_menu_slot_for_test(ConnectionId(1), 36, patched_gold());
    interact(&sender2, &receiver2, &mut restored, restored_id);
    assert!(held(&restored, restored_id, EquipmentSlot::OffHand).is_some());
    chat_command(
        &sender2,
        &receiver2,
        &mut restored,
        1,
        &format!("damage {uuid} 1 minecraft:mob_attack by {uuid}"),
    );
    assert!(held(&restored, restored_id, EquipmentSlot::OffHand).is_some());
    chat_command(
        &sender2,
        &receiver2,
        &mut restored,
        1,
        &format!("damage {uuid} 2 minecraft:player_attack by alice"),
    );
    assert!(held(&restored, restored_id, EquipmentSlot::OffHand).is_none());
    interact(&sender2, &receiver2, &mut restored, restored_id);
    assert!(held(&restored, restored_id, EquipmentSlot::OffHand).is_none());
    for _ in 0..405 {
        restored.tick(receiver2.drain());
    }
    assert_eq!(diamonds(&restored).len(), 1);
    let body = restored
        .stored_entity_mut_for_test(DIM, restored_id)
        .unwrap();
    body.set_pos(AT);
    body.set_delta_movement(Vec3d::default());
    interact(&sender2, &receiver2, &mut restored, restored_id);
    assert!(held(&restored, restored_id, EquipmentSlot::OffHand).is_some());
}

#[test]
fn gold_nuggets_take_the_whole_stack_and_throw_only_inventory_overflow() {
    for (item, space, loved_offhand, baby, repel) in [
        ("minecraft:gold_nugget", true, true, false, false),
        ("minecraft:gold_nugget", false, true, false, false),
        ("minecraft:raw_gold", false, false, false, false),
        ("minecraft:raw_gold", true, true, false, false),
        ("minecraft:clay_ball", true, false, true, true),
        ("minecraft:clay_ball", true, false, false, true),
    ] {
        let mut cfg = config(GameType::Creative);
        if repel {
            let tags = &mut Arc::make_mut(cfg.datapacks.as_mut().unwrap())
                .tags
                .registries
                .iter_mut()
                .find(|registry| registry.registry_id == "minecraft:item")
                .unwrap()
                .tags;
            tags.get_mut(if baby {
                "minecraft:ignored_by_piglin_babies"
            } else {
                "minecraft:piglin_repellents"
            })
            .unwrap()
            .push(item.into());
        }
        let candidate = restore_candidate(0, None, Vec::new());
        let mut world = spawn_pinned_restore(cfg, &candidate);
        let (sender, receiver) = mailbox();
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        let (id, _) = spawn(&mut world, baby);
        let mut gold = patched_gold();
        gold.item_id = Some(item_id(item));
        let stack = RuntimeItemStack::from_summary(gold.clone());
        let body = world.stored_entity_mut_for_test(DIM, id).unwrap();
        body.living_mut().unwrap().set_item_slot(
            EquipmentSlot::MainHand,
            Some(crate::entity::item::plain_stack("minecraft:golden_sword", 1).unwrap()),
        );
        if loved_offhand {
            body.living_mut().unwrap().set_item_slot(
                EquipmentSlot::OffHand,
                Some(crate::entity::item::plain_stack("minecraft:gold_ingot", 1).unwrap()),
            );
            crate::entity::set_spawner_brain_memory(
                body,
                PackedMemory {
                    memory_type: MemoryModuleType::AdmiringItem,
                    value: MemoryValue::Bool(true),
                    time_to_live: Some(119),
                },
            );
        }
        let NbtSummaryValue::Compound(mut record) =
            world.stored_entity_command_data_for_test(DIM, id).unwrap()
        else {
            unreachable!()
        };
        let inventory = (0..8)
            .map(|slot| {
                let item = if slot == 7 && space {
                    stack.with_count(63)
                } else {
                    crate::entity::item::plain_stack("minecraft:stone", 64).unwrap()
                };
                bbb_registry_codec::item_stack_summary_to_persistent_nbt(item.summary()).unwrap()
            })
            .collect();
        record.retain(|entry| entry.name != "Inventory");
        record.push(nbt("Inventory", NbtSummaryValue::List(inventory)));
        world.load_stored_entity_record_for_test(DIM, id, &record);
        let ground = world.spawn_ground_item_for_test(DIM, AT, &gold);
        let tick = world.tick(receiver.drain());
        let accepted = item == "minecraft:gold_nugget" && space;
        assert_eq!(
            tick.events
                .iter()
                .any(|event| matches!(event, WorldEvent::Send {
            packet: ClientboundPlayPacket::TakeItemEntity(take), ..
        } if take.item_id == ground && take.player_id == id && take.amount == 3)),
            accepted
        );
        if accepted {
            assert!(world.stored_entity_for_test(DIM, ground).is_none());
            let EntityData::GeneralLiving(data) =
                world.stored_entity_for_test(DIM, id).unwrap().data()
            else {
                unreachable!()
            };
            assert_eq!(
                data.internal_inventory().unwrap().item(7).unwrap().count(),
                64
            );
            let dropped = world
                .stored_entity_ids_for_test(DIM)
                .into_iter()
                .find_map(
                    |id| match world.stored_entity_for_test(DIM, id).unwrap().data() {
                        EntityData::Item(data) if data.item().item_resource_id() == Some(item) => {
                            Some(data.item().clone())
                        }
                        _ => None,
                    },
                )
                .unwrap();
            assert_eq!(dropped.count(), 2);
            assert_eq!(
                dropped.summary().component_patch.custom_data,
                gold.component_patch.custom_data
            );
            assert_eq!(
                held(&world, id, EquipmentSlot::OffHand)
                    .unwrap()
                    .item_resource_id(),
                Some("minecraft:gold_ingot")
            );
        } else {
            let EntityData::Item(data) = world.stored_entity_for_test(DIM, ground).unwrap().data()
            else {
                unreachable!()
            };
            assert_eq!(data.item().count(), 3);
        }
    }
}

fn ground_count(world: &ServerWorld, id: i32) -> i32 {
    match world.stored_entity_for_test(DIM, id).unwrap().data() {
        EntityData::Item(data) => data.item().count(),
        _ => unreachable!(),
    }
}

fn place_at_pickup(world: &mut ServerWorld, id: i32) {
    let body = world.stored_entity_mut_for_test(DIM, id).unwrap();
    body.set_pos(AT);
    body.set_delta_movement(Vec3d::default());
}

use super::helpers::set_no_ai;

fn fill_inventory(world: &mut ServerWorld, id: i32, slots: usize) {
    let NbtSummaryValue::Compound(mut record) =
        world.stored_entity_command_data_for_test(DIM, id).unwrap()
    else {
        unreachable!()
    };
    let stone = crate::entity::item::plain_stack("minecraft:stone", 64).unwrap();
    let inventory = (0..slots)
        .map(|_| bbb_registry_codec::item_stack_summary_to_persistent_nbt(stone.summary()).unwrap())
        .collect();
    record.retain(|entry| entry.name != "Inventory");
    record.push(nbt("Inventory", NbtSummaryValue::List(inventory)));
    world.load_stored_entity_record_for_test(DIM, id, &record);
}

#[test]
fn food_pickup_consumes_one_without_equipping_and_cooldown_pauses_without_ai() {
    for baby in [false, true] {
        let (_, receiver, mut world, _) = fixture(GameType::Creative);
        let (id, _) = spawn(&mut world, baby);
        let sword = crate::entity::item::plain_stack("minecraft:golden_sword", 1).unwrap();
        let shield = crate::entity::item::plain_stack("minecraft:shield", 1).unwrap();
        let body = world.stored_entity_mut_for_test(DIM, id).unwrap();
        body.living_mut().unwrap().set_health(10.0);
        body.living_mut()
            .unwrap()
            .set_item_slot(EquipmentSlot::MainHand, Some(sword.clone()));
        body.living_mut()
            .unwrap()
            .set_item_slot(EquipmentSlot::OffHand, Some(shield.clone()));
        set_no_ai(&mut world, id, true);
        let EntityData::GeneralLiving(data) = world.stored_entity_for_test(DIM, id).unwrap().data()
        else {
            unreachable!()
        };
        let before_inventory = data.internal_inventory().unwrap().slots().to_vec();
        let mut food = patched_gold();
        food.item_id = Some(item_id("minecraft:porkchop"));
        let ground = world.spawn_ground_item_for_test(DIM, AT, &food);
        let tick = world.tick(receiver.drain());
        assert!(tick
            .events
            .iter()
            .any(|event| matches!(event, WorldEvent::Send {
            packet: ClientboundPlayPacket::TakeItemEntity(take), ..
        } if take.item_id == ground && take.player_id == id && take.amount == 1)));
        assert_eq!(ground_count(&world, ground), 2);
        for _ in 0..220 {
            place_at_pickup(&mut world, ground);
            world.tick(receiver.drain());
        }
        assert_eq!(ground_count(&world, ground), 2);
        set_no_ai(&mut world, id, false);
        for _ in 0..175 {
            place_at_pickup(&mut world, id);
            place_at_pickup(&mut world, ground);
            world.tick(receiver.drain());
        }
        assert_eq!(ground_count(&world, ground), 2);
        for _ in 0..35 {
            place_at_pickup(&mut world, id);
            place_at_pickup(&mut world, ground);
            world.tick(receiver.drain());
        }
        assert_eq!(ground_count(&world, ground), 1);
        assert_eq!(held(&world, id, EquipmentSlot::MainHand), Some(sword));
        assert_eq!(held(&world, id, EquipmentSlot::OffHand), Some(shield));
        let body = world.stored_entity_for_test(DIM, id).unwrap();
        assert_eq!(body.living().unwrap().health(), 10.0);
        let EntityData::GeneralLiving(data) = body.data() else {
            unreachable!()
        };
        assert_eq!(data.internal_inventory().unwrap().slots(), before_inventory);
        let EntityData::Item(data) = world.stored_entity_for_test(DIM, ground).unwrap().data()
        else {
            unreachable!()
        };
        assert_eq!(
            data.item().summary().component_patch.custom_data,
            food.component_patch.custom_data
        );
    }
}

#[test]
fn food_uses_loaded_tag_requires_inventory_space_and_cooldown_is_not_saved() {
    let mut cfg = config(GameType::Creative);
    Arc::make_mut(cfg.datapacks.as_mut().unwrap())
        .tags
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:item")
        .unwrap()
        .tags
        .get_mut("minecraft:piglin_food")
        .unwrap()
        .push("minecraft:stick".into());
    let mut candidate = restore_candidate(0, None, Vec::new());
    let mut world = spawn_pinned_restore(cfg.clone(), &candidate);
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let (id, uuid) = spawn(&mut world, false);
    set_no_ai(&mut world, id, true);
    fill_inventory(&mut world, id, 8);
    let mut food = patched_gold();
    food.item_id = Some(item_id("minecraft:stick"));
    let ground = world.spawn_ground_item_for_test(DIM, AT, &food);
    world.tick(receiver.drain());
    assert_eq!(ground_count(&world, ground), 3);
    fill_inventory(&mut world, id, 7);
    place_at_pickup(&mut world, ground);
    world.tick(receiver.drain());
    assert_eq!(ground_count(&world, ground), 2);
    let EntityData::GeneralLiving(data) = world.stored_entity_for_test(DIM, id).unwrap().data()
    else {
        unreachable!()
    };
    assert!(data.internal_inventory().unwrap().item(7).is_none());
    assert!(held(&world, id, EquipmentSlot::MainHand).is_none());
    assert!(held(&world, id, EquipmentSlot::OffHand).is_none());
    place_at_pickup(&mut world, ground);
    world.tick(receiver.drain());
    assert_eq!(ground_count(&world, ground), 2);
    world
        .stored_entity_mut_for_test(DIM, ground)
        .unwrap()
        .set_pos(Vec3d { x: 10.5, ..AT });
    for (dimension, chunk) in world.pending_runtime_entity_chunks() {
        world
            .complete_runtime_entity_chunk(&dimension, chunk, None)
            .unwrap();
    }
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let mut restored = spawn_pinned_restore(cfg, &candidate);
    let restored_id = restored
        .live_entity_id_for_uuid_for_test(DIM, uuid)
        .unwrap();
    let (sender2, receiver2) = mailbox();
    join_loaded(&sender2, &receiver2, &mut restored, 1, "alice", 1);
    let new_ground = restored.spawn_ground_item_for_test(DIM, AT, &food);
    let tick = restored.tick(receiver2.drain());
    assert!(tick
        .events
        .iter()
        .any(|event| matches!(event, WorldEvent::Send {
        packet: ClientboundPlayPacket::TakeItemEntity(take), ..
    } if take.player_id == restored_id && take.amount == 1)));
    assert_eq!(ground_count(&restored, new_ground), 2);
    let EntityData::GeneralLiving(data) = restored
        .stored_entity_for_test(DIM, restored_id)
        .unwrap()
        .data()
    else {
        unreachable!()
    };
    assert!(data.internal_inventory().unwrap().item(7).is_none());
}

#[test]
fn ordinary_ground_equipment_replaces_its_slot_and_refuses_a_worse_helmet() {
    let (_, receiver, mut world, _) = fixture(GameType::Creative);
    let (id, _) = spawn(&mut world, false);
    let old = crate::entity::item::plain_stack("minecraft:iron_helmet", 1).unwrap();
    world
        .stored_entity_mut_for_test(DIM, id)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_item_slot(EquipmentSlot::Head, Some(old.clone()));
    let NbtSummaryValue::Compound(mut record) =
        world.stored_entity_command_data_for_test(DIM, id).unwrap()
    else {
        unreachable!()
    };
    record.retain(|entry| {
        entry.name != "NoAI" && entry.name != "PersistenceRequired" && entry.name != "drop_chances"
    });
    record.extend([
        nbt("NoAI", NbtSummaryValue::Byte(1)),
        nbt("PersistenceRequired", NbtSummaryValue::Byte(0)),
        nbt(
            "drop_chances",
            NbtSummaryValue::Compound(vec![nbt("head", NbtSummaryValue::Float(1.0f32.to_bits()))]),
        ),
    ]);
    world.load_stored_entity_record_for_test(DIM, id, &record);
    let mut helmet = patched_gold();
    helmet.item_id = Some(item_id("minecraft:diamond_helmet"));
    let ground = world.spawn_ground_item_for_test(DIM, AT, &helmet);
    let tick = world.tick(receiver.drain());
    assert!(tick
        .events
        .iter()
        .any(|event| matches!(event, WorldEvent::Send {
        packet: ClientboundPlayPacket::TakeItemEntity(take), ..
    } if take.item_id == ground && take.player_id == id && take.amount == 1)));
    assert_eq!(
        held(&world, id, EquipmentSlot::Head),
        Some(RuntimeItemStack::from_summary(helmet.clone()).with_count(1))
    );
    assert_eq!(ground_count(&world, ground), 2);
    let EntityData::Item(data) = world.stored_entity_for_test(DIM, ground).unwrap().data() else {
        unreachable!()
    };
    assert_eq!(
        data.item().summary().component_patch.custom_data,
        helmet.component_patch.custom_data
    );
    assert!(persistent(&world, id));
    assert!(world.stored_entity_ids_for_test(DIM).into_iter().any(|other| matches!(world.stored_entity_for_test(DIM, other).unwrap().data(), EntityData::Item(data) if data.item() == &old)));
    world
        .stored_entity_mut_for_test(DIM, ground)
        .unwrap()
        .set_pos(Vec3d { x: 10.5, ..AT });
    let mut worse = helmet.clone();
    worse.item_id = Some(item_id("minecraft:iron_helmet"));
    let worse_ground = world.spawn_ground_item_for_test(DIM, AT, &worse);
    let tick = world.tick(receiver.drain());
    assert!(!tick
        .events
        .iter()
        .any(|event| matches!(event, WorldEvent::Send {
        packet: ClientboundPlayPacket::TakeItemEntity(take), ..
    } if take.item_id == worse_ground)));
    assert_eq!(
        held(&world, id, EquipmentSlot::Head),
        Some(RuntimeItemStack::from_summary(helmet).with_count(1))
    );
    assert!(held(&world, id, EquipmentSlot::MainHand).is_none());
    assert_eq!(ground_count(&world, worse_ground), 3);
}

#[test]
fn visible_loved_ground_item_is_approached_and_picked_up_unless_admiring_is_disabled() {
    for disabled in [false, true] {
        let (sender, receiver, mut world, _) = fixture(GameType::Creative);
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "fill -3 64 -3 12 64 3 minecraft:stone",
        );
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "fill -3 65 -3 12 68 3 minecraft:air",
        );
        let (id, _) = spawn(&mut world, false);
        let body = world.stored_entity_mut_for_test(DIM, id).unwrap();
        body.set_no_gravity(false);
        body.living_mut()
            .unwrap()
            .set_item_slot(EquipmentSlot::MainHand, None);
        if disabled {
            crate::entity::set_spawner_brain_memory(
                body,
                PackedMemory {
                    memory_type: MemoryModuleType::AdmiringDisabled,
                    value: MemoryValue::Bool(true),
                    time_to_live: Some(400),
                },
            );
        }
        let mut stack = patched_gold();
        stack.item_id = Some(item_id("minecraft:clay_ball"));
        let target = Vec3d {
            x: AT.x + 5.0,
            ..AT
        };
        let ground = world.spawn_ground_item_for_test(DIM, target, &stack);
        for _ in 0..50 {
            world.tick(receiver.drain());
            if !disabled && world.stored_entity_for_test(DIM, id).unwrap().position().x > AT.x + 1.0
            {
                break;
            }
        }
        let position = world.stored_entity_for_test(DIM, id).unwrap().position();
        if disabled {
            assert!(
                position.x < AT.x + 1.0,
                "disabled piglin approached: {position:?}"
            );
            assert_eq!(ground_count(&world, ground), 3);
            assert!(held(&world, id, EquipmentSlot::OffHand).is_none());
            continue;
        }
        assert!(
            position.x > AT.x + 1.0,
            "piglin did not approach: {position:?}"
        );
        let mut taken = false;
        for _ in 0..65 {
            let tick = world.tick(receiver.drain());
            taken |= tick.events.iter().any(|event| {
                matches!(event, WorldEvent::Send {
                packet: ClientboundPlayPacket::TakeItemEntity(take), ..
            } if take.item_id == ground && take.player_id == id && take.amount == 1)
            });
            if taken {
                break;
            }
        }
        assert!(taken);
        assert_eq!(ground_count(&world, ground), 2);
        assert_eq!(
            held(&world, id, EquipmentSlot::OffHand),
            Some(RuntimeItemStack::from_summary(stack).with_count(1))
        );
    }
}

fn prepare_protected_container(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    container: &str,
) {
    chat_command(
        sender,
        receiver,
        world,
        1,
        "fill -2 64 -3 12 64 6 minecraft:stone",
    );
    chat_command(
        sender,
        receiver,
        world,
        1,
        "fill -2 65 -3 12 69 6 minecraft:air",
    );
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 0.5,
            y: 65.0,
            z: 0.5,
        },
    );
    world.stock_menu_slot_for_test(
        ConnectionId(1),
        5,
        ItemStackSummary {
            item_id: Some(item_id("minecraft:golden_helmet")),
            count: 1,
            component_patch: Default::default(),
        },
    );
    assert!(world.set_block(
        DIM,
        BlockPos { x: 0, y: 65, z: 1 },
        helpers::default_block_state(container),
        SET_BLOCK_UPDATE_ALL
    ));
}

fn open_protected_container(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos: BlockPos { x: 0, y: 65, z: 1 },
                direction: Direction::North,
                cursor_x: 0.5,
                cursor_y: 0.5,
                cursor_z: 0.0,
                inside: false,
                world_border_hit: false,
            },
            sequence: 1,
        }),
    )
}

fn arm_piglin(world: &mut ServerWorld, id: i32, position: Vec3d) {
    let body = world.stored_entity_mut_for_test(DIM, id).unwrap();
    body.set_pos(position);
    body.set_no_gravity(false);
    body.living_mut().unwrap().set_item_slot(
        EquipmentSlot::MainHand,
        Some(crate::entity::item::plain_stack("minecraft:golden_sword", 1).unwrap()),
    );
}

#[test]
fn brutes_pursue_and_hurt_gold_armored_players_only_within_follow_range() {
    for distance in [8.0, 14.0] {
        let (sender, receiver, mut world, _) = fixture(GameType::Survival);
        prepare_protected_container(&sender, &receiver, &mut world, "minecraft:chest");
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "fill 13 64 -3 16 64 6 minecraft:stone",
        );
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "fill 13 65 -3 16 69 6 minecraft:air",
        );
        let start = Vec3d {
            x: 0.5 + distance,
            ..AT
        };
        let (id, uuid) = world.spawn_stored_entity_for_test(
            DIM,
            VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
            start,
            EntityData::GeneralLiving(
                GeneralLivingData::new(VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID).unwrap(),
            ),
        );
        let NbtSummaryValue::Compound(mut record) =
            world.stored_entity_command_data_for_test(DIM, id).unwrap()
        else {
            unreachable!()
        };
        for name in ["IsImmuneToZombification", "PersistenceRequired"] {
            record.retain(|entry| entry.name != name);
            record.push(nbt(name, NbtSummaryValue::Byte(1)));
        }
        world.load_stored_entity_record_for_test(DIM, id, &record);
        if distance > 12.0 {
            chat_command(
                &sender,
                &receiver,
                &mut world,
                1,
                &format!("attribute {uuid} minecraft:movement_speed base set 0"),
            );
        }
        for _ in 0..100 {
            world.tick(receiver.drain());
            if world.player_health_for_test(ConnectionId(1)) < 20.0 {
                break;
            }
        }
        let end = world.stored_entity_for_test(DIM, id).unwrap().position();
        if distance < 12.0 {
            assert!(end.x < start.x - 1.0);
            assert!(world.player_health_for_test(ConnectionId(1)) < 20.0);
        } else {
            assert_eq!(end.x, start.x);
            assert_eq!(world.player_health_for_test(ConnectionId(1)), 20.0);
            assert!(world.mob_target_for_test(DIM, id).is_none());
        }
    }
}

fn spawn_nemesis(world: &mut ServerWorld, entity_type: i32, position: Vec3d) -> i32 {
    let (id, _) = world.spawn_stored_entity_for_test(
        DIM,
        entity_type,
        position,
        EntityData::GeneralLiving(GeneralLivingData::new(entity_type).unwrap()),
    );
    let NbtSummaryValue::Compound(mut record) =
        world.stored_entity_command_data_for_test(DIM, id).unwrap()
    else {
        unreachable!()
    };
    for (name, value) in [("NoAI", 1), ("NoGravity", 1), ("PersistenceRequired", 1)] {
        record.retain(|entry| entry.name != name);
        record.push(nbt(name, NbtSummaryValue::Byte(value)));
    }
    world.load_stored_entity_record_for_test(DIM, id, &record);
    id
}

#[test]
fn adults_pursue_and_hurt_visible_nemeses_even_without_a_universal_anger_player() {
    for (entity_type, universal) in [
        (VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID, false),
        (VANILLA_ENTITY_TYPE_WITHER_ID, true),
    ] {
        let (sender, receiver, mut world, _) = fixture(GameType::Creative);
        prepare_protected_container(&sender, &receiver, &mut world, "minecraft:chest");
        let target = spawn_nemesis(&mut world, entity_type, AT);
        let health = world
            .stored_entity_for_test(DIM, target)
            .unwrap()
            .living()
            .unwrap()
            .health();
        let (id, _) = spawn(&mut world, false);
        let start = Vec3d { x: 10.5, ..AT };
        arm_piglin(&mut world, id, start);
        if universal {
            crate::entity::set_spawner_brain_memory(
                world.stored_entity_mut_for_test(DIM, id).unwrap(),
                PackedMemory {
                    memory_type: MemoryModuleType::UniversalAnger,
                    value: MemoryValue::Bool(true),
                    time_to_live: Some(600),
                },
            );
        }
        for _ in 0..100 {
            world.tick(receiver.drain());
            if world
                .stored_entity_for_test(DIM, target)
                .unwrap()
                .living()
                .unwrap()
                .health()
                < health
            {
                break;
            }
        }
        assert!(world.stored_entity_for_test(DIM, id).unwrap().position().x < start.x - 1.0);
        assert!(
            world
                .stored_entity_for_test(DIM, target)
                .unwrap()
                .living()
                .unwrap()
                .health()
                < health
        );
    }
}

fn anger(world: &ServerWorld, id: i32) -> Option<&PackedMemory> {
    crate::entity::brain_packed(world.stored_entity_for_test(DIM, id)?)?
        .memories
        .iter()
        .find(|memory| memory.memory_type == MemoryModuleType::AngryAt)
}

#[test]
fn adults_target_players_without_loaded_safe_armor_but_babies_do_not() {
    for (safe_armor, baby) in [
        ("minecraft:golden_helmet", false),
        ("minecraft:iron_helmet", false),
        ("minecraft:golden_helmet", true),
    ] {
        let mut config = config(GameType::Survival);
        if safe_armor == "minecraft:iron_helmet" {
            *Arc::make_mut(config.datapacks.as_mut().unwrap())
                .tags
                .registries
                .iter_mut()
                .find(|registry| registry.registry_id == "minecraft:item")
                .unwrap()
                .tags
                .get_mut("minecraft:piglin_safe_armor")
                .unwrap() = vec![safe_armor.into()];
        }
        let (sender, receiver, mut world, _) = fixture_with_config(config);
        prepare_protected_container(&sender, &receiver, &mut world, "minecraft:chest");
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("item replace entity @s armor.head with {safe_armor}"),
        );
        let (id, _) = spawn(&mut world, baby);
        let start = Vec3d { x: 10.5, ..AT };
        arm_piglin(&mut world, id, start);
        for _ in 0..40 {
            world.tick(receiver.drain());
        }
        assert_eq!(world.player_health_for_test(ConnectionId(1)), 20.0);
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "item replace entity @s armor.head with minecraft:air",
        );
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("item replace entity @s weapon.mainhand with {safe_armor}"),
        );
        if safe_armor == "minecraft:golden_helmet" && !baby {
            for _ in 0..20 {
                world.tick(receiver.drain());
            }
            arm_piglin(&mut world, id, Vec3d { x: 12.5, ..AT });
            world.set_player_position_for_test(ConnectionId(1), Vec3d { x: -1.5, ..AT });
            chat_command(
                &sender,
                &receiver,
                &mut world,
                1,
                "effect give @s minecraft:invisibility 10 0 true",
            );
        }
        for _ in 0..100 {
            world.tick(receiver.drain());
            if world.player_health_for_test(ConnectionId(1)) < 20.0 {
                break;
            }
        }
        assert!(anger(&world, id).is_none());
        if baby {
            assert_eq!(world.player_health_for_test(ConnectionId(1)), 20.0);
            continue;
        }
        assert!(world.player_health_for_test(ConnectionId(1)) < 20.0);
        assert!(world.stored_entity_for_test(DIM, id).unwrap().position().x < start.x - 1.0);
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("item replace entity @s armor.head with {safe_armor}"),
        );
        world.set_player_health_for_test(ConnectionId(1), 20.0);
        for _ in 0..40 {
            world.tick(receiver.drain());
        }
        assert_eq!(world.player_health_for_test(ConnectionId(1)), 20.0);
    }
}

#[test]
fn protected_containers_provoke_armed_adults_to_pursue_and_hurt_a_gold_armored_player() {
    for container in [
        "minecraft:chest",
        "minecraft:barrel",
        "minecraft:shulker_box",
        "minecraft:ender_chest",
    ] {
        let (sender, receiver, mut world, _) = fixture(GameType::Survival);
        prepare_protected_container(&sender, &receiver, &mut world, container);
        let (id, _) = spawn(&mut world, false);
        let start = Vec3d { x: 6.5, ..AT };
        arm_piglin(&mut world, id, start);
        let nemesis = (container == "minecraft:chest").then(|| {
            spawn_nemesis(
                &mut world,
                VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID,
                Vec3d { x: 9.5, ..AT },
            )
        });
        let tick = open_protected_container(&sender, &receiver, &mut world);
        assert!(
            tick.events.iter().any(|event| matches!(
                event,
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::OpenScreen(_),
                    ..
                }
            )),
            "{container}"
        );
        assert!(
            matches!(anger(&world, id), Some(PackedMemory {
            value: MemoryValue::Uuid(uuid), time_to_live: Some(590..=600), ..
        }) if *uuid == Uuid::from_u128(1)),
            "{container}"
        );
        if container == "minecraft:chest" {
            chat_command(
                &sender,
                &receiver,
                &mut world,
                1,
                "effect give @s minecraft:invisibility 10 0 true",
            );
        }
        for _ in 0..100 {
            world.tick(receiver.drain());
            if world.player_health_for_test(ConnectionId(1)) < 20.0 {
                break;
            }
        }
        assert!(
            world.stored_entity_for_test(DIM, id).unwrap().position().x < start.x - 1.0,
            "{container}"
        );
        assert!(
            world.player_health_for_test(ConnectionId(1)) < 20.0,
            "{container}"
        );
        if let Some(nemesis) = nemesis {
            assert_eq!(
                world
                    .stored_entity_for_test(DIM, nemesis)
                    .unwrap()
                    .living()
                    .unwrap()
                    .health(),
                20.0
            );
        }
    }
}

#[test]
fn opening_requires_sight_and_idle_piglins_and_does_not_make_babies_attack() {
    let (sender, receiver, mut world, _) = fixture(GameType::Survival);
    prepare_protected_container(&sender, &receiver, &mut world, "minecraft:chest");
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "fill 1 65 2 8 68 2 minecraft:stone",
    );
    let (hidden, _) = spawn(&mut world, false);
    arm_piglin(
        &mut world,
        hidden,
        Vec3d {
            x: 5.5,
            z: 4.5,
            ..AT
        },
    );
    let (baby, _) = spawn(&mut world, true);
    arm_piglin(&mut world, baby, Vec3d { x: 3.5, ..AT });
    let (admiring, _) = spawn(&mut world, false);
    arm_piglin(&mut world, admiring, Vec3d { x: 6.5, ..AT });
    crate::entity::set_spawner_brain_memory(
        world.stored_entity_mut_for_test(DIM, admiring).unwrap(),
        PackedMemory {
            memory_type: MemoryModuleType::AdmiringItem,
            value: MemoryValue::Bool(true),
            time_to_live: Some(119),
        },
    );
    world
        .stored_entity_mut_for_test(DIM, admiring)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_item_slot(
            EquipmentSlot::OffHand,
            Some(crate::entity::item::plain_stack("minecraft:gold_ingot", 1).unwrap()),
        );
    open_protected_container(&sender, &receiver, &mut world);
    assert!(anger(&world, hidden).is_none());
    assert!(anger(&world, admiring).is_none());
    assert!(anger(&world, baby).is_some());
    for _ in 0..60 {
        world.tick(receiver.drain());
    }
    assert_eq!(world.player_health_for_test(ConnectionId(1)), 20.0);
}

#[test]
fn protected_action_anger_restores_its_remaining_lifetime_and_stops_attacking_after_expiry() {
    let (sender, receiver, mut world, mut candidate) = fixture(GameType::Survival);
    prepare_protected_container(&sender, &receiver, &mut world, "minecraft:chest");
    let (id, uuid) = spawn(&mut world, false);
    arm_piglin(&mut world, id, Vec3d { x: 6.5, ..AT });
    open_protected_container(&sender, &receiver, &mut world);
    for _ in 0..25 {
        world.tick(receiver.drain());
    }
    let remaining = anger(&world, id).unwrap().time_to_live.unwrap();
    for (dimension, chunk) in world.pending_runtime_entity_chunks() {
        world
            .complete_runtime_entity_chunk(&dimension, chunk, None)
            .unwrap();
    }
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    restore_entity_writes(&mut candidate, &request.entity_writes);
    for write in &request.chunk_writes {
        let data = Java26ChunkData::parse(
            write.document.clone(),
            Java26ChunkContext {
                expected_x: write.chunk_x,
                expected_z: write.chunk_z,
                ..restore_chunk_context()
            },
        )
        .unwrap();
        if data.status() == bbb_server_persistence::Java26ChunkStatus::Full {
            candidate
                .dimensions
                .get_mut(&write.dimension)
                .unwrap()
                .chunks
                .insert(
                    (write.chunk_x, write.chunk_z),
                    Java26LoadedChunk {
                        timestamp: 200,
                        data,
                    },
                );
        }
    }
    world = spawn_pinned_restore(config(GameType::Survival), &candidate);
    let id = world.live_entity_id_for_uuid_for_test(DIM, uuid).unwrap();
    assert_eq!(anger(&world, id).unwrap().time_to_live, Some(remaining));
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 0.5,
            y: 65.0,
            z: 0.5,
        },
    );
    for _ in 0..100 {
        world.tick(receiver.drain());
        if world.player_health_for_test(ConnectionId(1)) < 20.0 {
            break;
        }
    }
    assert!(world.player_health_for_test(ConnectionId(1)) < 20.0);
    chat_command(&sender, &receiver, &mut world, 1, "gamemode creative");
    for _ in 0..remaining + 2 {
        world.tick(receiver.drain());
    }
    assert!(anger(&world, id).is_none());
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "item replace entity @s armor.head with minecraft:golden_helmet",
    );
    chat_command(&sender, &receiver, &mut world, 1, "gamemode survival");
    world.set_player_health_for_test(ConnectionId(1), 20.0);
    for _ in 0..40 {
        world.tick(receiver.drain());
    }
    assert_eq!(world.player_health_for_test(ConnectionId(1)), 20.0);
    // Idle strolling may have left the 16-block alert box.
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("tp {uuid} 6.5 65 0.5"),
    );
    open_protected_container(&sender, &receiver, &mut world);
    assert!(anger(&world, id).is_some());
    world.set_player_health_for_test(ConnectionId(1), 1.0);
    for _ in 0..100 {
        world.tick(receiver.drain());
        if world.player_health_for_test(ConnectionId(1)) == 0.0 {
            break;
        }
    }
    assert_eq!(world.player_health_for_test(ConnectionId(1)), 0.0);
    world.tick(receiver.drain());
    assert!(anger(&world, id).is_none());
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ClientCommand(ClientCommandAction::PerformRespawn),
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerLoaded,
    );
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "item replace entity @s armor.head with minecraft:golden_helmet",
    );
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 0.5,
            y: 65.0,
            z: 0.5,
        },
    );
    for _ in 0..40 {
        world.tick(receiver.drain());
    }
    assert_eq!(world.player_health_for_test(ConnectionId(1)), 20.0);
}
