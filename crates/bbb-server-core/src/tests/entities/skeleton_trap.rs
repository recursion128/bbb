use super::*;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID, VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
    VANILLA_ENTITY_TYPE_SKELETON_ID,
};
use bbb_protocol::equipment_slots::EquipmentSlot;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    text: &str,
) -> TickResult {
    let operation_id = world.publication().tick;
    let tick = console_command(sender, receiver, world, operation_id, text);
    let outcome = admin_outcome(&tick);
    assert!(!outcome.replayed, "{text}: fixture commands must execute");
    let Ok(AdminResult::Command(report)) = &outcome.result else {
        panic!("{text}: {outcome:?}");
    };
    assert!(report.result.is_some(), "{text}: {report:?}");
    tick
}

fn fixture_config() -> ServerWorldConfig {
    let mut config = datapack_config();
    config.keep_alive_interval_ticks = 100_000;
    let root = std::env::temp_dir().join(format!("bbb-provider-source-{}", uuid::Uuid::new_v4()));
    let pack = root.join("raw");
    let path = pack.join("data/minecraft/enchantment_provider/mob_spawn_equipment.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), br#"{"pack":{"description":"raw provider","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
    std::fs::write(
        path,
        br#"{"type":":single","enchantment":"minecraft:unbreaking","level":3,"\ud800":"\udc00"}"#,
    )
    .unwrap();
    let startup = bbb_server_datapack::startup::DatapackStartup::load(
        &bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: Some(root.clone()),
            saved_data_packs: None,
            safe_mode: false,
            limits: Default::default(),
        },
    )
    .unwrap();
    std::fs::remove_dir_all(root).unwrap();
    let provider = startup
        .registries
        .element(
            "minecraft:enchantment_provider",
            "minecraft:mob_spawn_equipment",
        )
        .unwrap();
    assert!(provider.source.is_some());
    assert!(provider.codec_value.is_none());
    config.datapacks = Some(Arc::new(startup));
    config
}

fn prepare(sender: &WorldCommandSender, receiver: &WorldCommandReceiver, world: &mut ServerWorld) {
    world.tick_rate_mut().set_frozen(true);
    join_loaded(sender, receiver, world, 1, "observer", 1);
    for text in [
        "gamerule spawn_mobs false",
        "gamerule advance_time false",
        "gamerule random_tick_speed 0",
        "time set midnight",
        "weather clear",
        "gamemode spectator observer",
        "tp observer 13.5 100 4.5",
        "fill 0 99 0 15 106 15 minecraft:stone",
        "fill 0 100 0 15 106 15 minecraft:air",
    ] {
        command(sender, receiver, world, text);
    }
}

fn ids(world: &ServerWorld, entity_type_id: i32) -> Vec<i32> {
    world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .filter(|&id| {
            world
                .stored_entity_for_test(DIM, id)
                .unwrap()
                .entity_type_id()
                == entity_type_id
        })
        .collect()
}

fn field(world: &ServerWorld, id: i32, name: &str) -> NbtSummaryValue {
    let record = world.stored_entity_command_data_for_test(DIM, id).unwrap();
    crate::loot_runtime::nbt_field(&record, name)
        .unwrap_or_else(|| panic!("missing {name}: {record:?}"))
        .clone()
}

fn normal_ticks(world: &mut ServerWorld, receiver: &WorldCommandReceiver, count: usize) {
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..count {
        world.tick(receiver.drain());
    }
    world.tick_rate_mut().set_frozen(true);
}

#[test]
fn skeleton_trap_uses_real_player_range_and_spawns_four_equipped_tracked_riders() {
    let (sender, receiver) = mailbox();
    let root = std::env::temp_dir().join(format!("bbb-skeleton-trap-{}", Uuid::new_v4()));
    let storage = created_fixture_save(&root, "Skeleton horse trap");
    let candidate = storage.load_candidate().unwrap();
    let mut world = spawn_pinned_restore(fixture_config(), &candidate);
    prepare(&sender, &receiver, &mut world);
    command(
        &sender,
        &receiver,
        &mut world,
        "summon minecraft:skeleton_horse 4.5 100 4.5 {SkeletonTrap:1b,SkeletonTrapTime:120,Age:-24000,NoAI:1b}",
    );
    let original = ids(&world, VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID)[0];
    let uuid = world.stored_entity_for_test(DIM, original).unwrap().uuid();
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("attribute {uuid} minecraft:movement_speed base set 0"),
    );
    normal_ticks(&mut world, &receiver, 40);
    assert_eq!(ids(&world, VANILLA_ENTITY_TYPE_SKELETON_ID).len(), 0);
    command(&sender, &receiver, &mut world, "gamemode creative observer");
    normal_ticks(&mut world, &receiver, 40);
    assert_eq!(ids(&world, VANILLA_ENTITY_TYPE_SKELETON_ID).len(), 0);
    for text in [
        "tp observer 14.5 100 4.5".to_owned(),
        format!("data merge entity {uuid} {{NoAI:0b}}"),
    ] {
        command(&sender, &receiver, &mut world, &text);
    }
    normal_ticks(&mut world, &receiver, 40);
    assert_eq!(ids(&world, VANILLA_ENTITY_TYPE_SKELETON_ID).len(), 0);
    assert_eq!(
        field(&world, original, "SkeletonTrap"),
        NbtSummaryValue::Byte(1)
    );
    for text in ["gamemode spectator observer", "tp observer 13.5 100 4.5"] {
        command(&sender, &receiver, &mut world, text);
    }
    normal_ticks(&mut world, &receiver, 40);
    assert_eq!(ids(&world, VANILLA_ENTITY_TYPE_SKELETON_ID).len(), 0);
    let timer_before = field(&world, original, "SkeletonTrapTime");
    command(&sender, &receiver, &mut world, "gamemode creative observer");
    let mut events = Vec::new();
    let mut visual_bolts = Vec::new();
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..40 {
        events.extend(world.tick(receiver.drain()).events);
        for id in ids(&world, VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID) {
            let crate::entity::EntityData::LightningBolt(data) =
                world.stored_entity_for_test(DIM, id).unwrap().data()
            else {
                unreachable!()
            };
            assert!(data.visual_only());
            if !visual_bolts.contains(&id) {
                visual_bolts.push(id);
            }
        }
        if ids(&world, VANILLA_ENTITY_TYPE_SKELETON_ID).len() == 4
            && world
                .game_events(DIM)
                .0
                .iter()
                .any(|event| event.game_event == VanillaGameEvent::LIGHTNING_STRIKE)
        {
            break;
        }
    }
    world.tick_rate_mut().set_frozen(true);
    assert_eq!(visual_bolts.len(), 1);
    assert!(world.game_events(DIM).0.iter().any(|event| {
        event.game_event == VanillaGameEvent::LIGHTNING_STRIKE
            && event.source_entity_id == Some(visual_bolts[0])
    }));
    let horses = ids(&world, VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID);
    let riders = ids(&world, VANILLA_ENTITY_TYPE_SKELETON_ID);
    assert_eq!(horses.len(), 4);
    assert_eq!(riders.len(), 4);
    assert_eq!(
        world.stored_entity_for_test(DIM, original).unwrap().uuid(),
        uuid
    );
    assert_eq!(
        field(&world, original, "SkeletonTrap"),
        NbtSummaryValue::Byte(0)
    );
    let NbtSummaryValue::Int(before) = timer_before else {
        unreachable!()
    };
    let NbtSummaryValue::Int(after) = field(&world, original, "SkeletonTrapTime") else {
        unreachable!()
    };
    assert!(
        after >= before && after > 0,
        "activation retains the existing timer"
    );
    assert_eq!(
        field(&world, original, "PersistenceRequired"),
        NbtSummaryValue::Byte(0)
    );
    for &horse in &horses {
        assert_eq!(field(&world, horse, "Tame"), NbtSummaryValue::Byte(1));
        assert_eq!(field(&world, horse, "Age"), NbtSummaryValue::Int(0));
        let passengers = world.stored_passengers_for_test(DIM, horse);
        assert_eq!(passengers.len(), 1);
        assert!(riders.contains(&passengers[0]));
        assert!(world
            .player_tracking_viewers_for_test(DIM, horse)
            .contains(&ConnectionId(1)));
        assert!(events.iter().any(|event| matches!(event,
            WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::SetPassengers(packet) }
            if packet.vehicle_id == horse && packet.passenger_ids == passengers
        )), "the observer receives horse {horse}'s actual passenger relation");
        if horse != original {
            assert_eq!(
                field(&world, horse, "PersistenceRequired"),
                NbtSummaryValue::Byte(1)
            );
        }
    }
    for rider in riders {
        assert_eq!(
            field(&world, rider, "PersistenceRequired"),
            NbtSummaryValue::Byte(1)
        );
        let living = world
            .stored_entity_for_test(DIM, rider)
            .unwrap()
            .living()
            .unwrap();
        for slot in [EquipmentSlot::Head, EquipmentSlot::MainHand] {
            let stack = living
                .equipment()
                .get(slot)
                .expect("both trap equipment slots are filled");
            assert_eq!(
                stack.component_nbt(
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
                    &world.item_stack_resolver(),
                ).unwrap(),
                Some(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "minecraft:unbreaking".into(),
                    value: NbtSummaryValue::Int(3),
                }]))
            );
        }
    }
    assert_eq!(events.iter().filter(|event| matches!(event,
        WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::AddEntity(packet) }
        if packet.entity_type_id == VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID
    )).count(), 1);
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn skeleton_trap_lifetime_runs_with_no_ai_and_preserves_persistent_or_inactive_timers() {
    let (sender, receiver) = mailbox();
    let root = std::env::temp_dir().join(format!("bbb-skeleton-trap-life-{}", Uuid::new_v4()));
    let storage = created_fixture_save(&root, "Skeleton trap lifetime");
    let candidate = storage.load_candidate().unwrap();
    let config = fixture_config();
    let mut world = spawn_pinned_restore(config.clone(), &candidate);
    prepare(&sender, &receiver, &mut world);
    let mut horses = Vec::new();
    for (x, extra) in [
        (4, "SkeletonTrap:1b,SkeletonTrapTime:17999"),
        (
            8,
            "SkeletonTrap:1b,SkeletonTrapTime:18010,PersistenceRequired:1b",
        ),
        (12, "SkeletonTrap:0b,SkeletonTrapTime:18010"),
    ] {
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("summon minecraft:skeleton_horse {x}.5 100 4.5 {{NoAI:1b,{extra}}}"),
        );
        let id = ids(&world, VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID)
            .into_iter()
            .find(|id| !horses.iter().any(|(other, _)| other == id))
            .unwrap();
        horses.push((id, world.stored_entity_for_test(DIM, id).unwrap().uuid()));
    }
    normal_ticks(&mut world, &receiver, 8);
    assert!(world
        .live_entity_id_for_uuid_for_test(DIM, horses[0].1)
        .is_none());
    assert_eq!(ids(&world, VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID).len(), 2);
    assert!(ids(&world, VANILLA_ENTITY_TYPE_SKELETON_ID).is_empty());
    for &(id, _) in &horses[1..] {
        assert_eq!(
            field(&world, id, "SkeletonTrapTime"),
            NbtSummaryValue::Int(18010)
        );
        assert_eq!(field(&world, id, "NoAI"), NbtSummaryValue::Byte(1));
    }
    let request = world
        .save_request(1_786_000_020_000, 220, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    let bootstrap = storage.load_bootstrap().unwrap();
    let saved = crate::world_creation::load_candidate_from_bootstrap(
        &storage,
        bootstrap,
        &config.datapacks.as_ref().unwrap().registries,
    )
    .unwrap();
    let restored = spawn_pinned_restore(config, &saved);
    assert!(restored
        .live_entity_id_for_uuid_for_test(DIM, horses[0].1)
        .is_none());
    for &(_, uuid) in &horses[1..] {
        let id = restored
            .live_entity_id_for_uuid_for_test(DIM, uuid)
            .unwrap();
        assert_eq!(
            field(&restored, id, "SkeletonTrapTime"),
            NbtSummaryValue::Int(18010)
        );
    }
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
