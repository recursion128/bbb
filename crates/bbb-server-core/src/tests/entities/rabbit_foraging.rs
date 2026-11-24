use super::*;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_RABBIT_ID;
use bbb_protocol::packets::SoundEventHolder;
use bbb_protocol::sound_event_types::vanilla_sound_event_resource_id;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const CROP: BlockPos = BlockPos {
    x: 12,
    y: 100,
    z: 8,
};
const START: Vec3d = Vec3d {
    x: 8.5,
    y: 100.0,
    z: 8.5,
};

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

fn carrot(age: &str) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:carrots",
        &BTreeMap::from([("age".into(), age.into())]),
    )
    .unwrap()
}

fn position(world: &ServerWorld, id: i32) -> Vec3d {
    world.stored_entity_for_test(DIM, id).unwrap().position()
}

fn cooldown(world: &ServerWorld, id: i32) -> i32 {
    let data = world.stored_entity_command_data_for_test(DIM, id).unwrap();
    let Some(NbtSummaryValue::Int(value)) =
        crate::loot_runtime::nbt_field(&data, "MoreCarrotTicks")
    else {
        panic!("rabbit has a saved food cooldown: {data:?}");
    };
    *value
}

fn horizontal_distance(a: Vec3d, b: Vec3d) -> f64 {
    (a.x - b.x).hypot(a.z - b.z)
}

fn jump_event(tick: &TickResult, id: i32) -> bool {
    tick.events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::EntityEvent(event) }
        if event.entity_id == id && event.event_id == 1
    ))
}

fn jump_sound(tick: &TickResult) -> bool {
    tick.events.iter().any(|event| {
        let sound = match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::Sound(sound),
            } => &sound.sound,
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::SoundEntity(sound),
            } => &sound.sound,
            _ => return false,
        };
        match sound {
            SoundEventHolder::Reference { registry_id } => {
                vanilla_sound_event_resource_id(*registry_id)
                    == Some("minecraft:entity.rabbit.jump")
            }
            SoundEventHolder::Direct { location, .. } => location == "minecraft:entity.rabbit.jump",
        }
    })
}

fn assert_no_drops(world: &ServerWorld) {
    assert!(
        world.stored_entity_ids_for_test(DIM).into_iter().all(|id| {
            !matches!(
                world.stored_entity_for_test(DIM, id).unwrap().data(),
                crate::entity::EntityData::Item(_)
            )
        }),
        "grazing does not create item entities"
    );
}

#[test]
fn rabbit_navigates_and_hops_to_ripe_carrots_then_saves_its_real_food_cooldown() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.keep_alive_interval_ticks = 100_000;
    let root = std::env::temp_dir().join(format!("bbb-rabbit-foraging-{}", uuid::Uuid::new_v4()));
    let storage = created_fixture_save(&root, "Rabbit foraging");
    let candidate = storage.load_candidate().unwrap();
    let mut world = spawn_pinned_restore(config.clone(), &candidate);
    world.tick_rate_mut().set_frozen(true);
    join_loaded(&sender, &receiver, &mut world, 1, "observer", 1);
    for text in [
        "gamerule spawn_mobs false",
        "gamerule advance_time false",
        "gamerule random_tick_speed 0",
        "time set day",
        "weather clear",
        "gamemode spectator observer",
        "tp observer 10.5 105 8.5",
        "fill 5 99 5 15 103 11 minecraft:stone",
        "fill 6 100 6 14 103 10 minecraft:air",
        "setblock 12 99 8 minecraft:farmland[moisture=7]",
        "setblock 12 100 8 minecraft:carrots[age=7]",
        "summon minecraft:rabbit 8.5 100 8.5 {NoAI:1b,PersistenceRequired:1b}",
    ] {
        command(&sender, &receiver, &mut world, text);
    }
    let id = world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(DIM, *id)
                .unwrap()
                .entity_type_id()
                == VANILLA_ENTITY_TYPE_RABBIT_ID
        })
        .unwrap();
    let rabbit = world.stored_entity_for_test(DIM, id).unwrap();
    let uuid = rabbit.uuid();
    let health = rabbit.living().unwrap().health();
    assert_eq!(cooldown(&world, id), 0);
    assert!(world
        .player_tracking_viewers_for_test(DIM, id)
        .contains(&ConnectionId(1)));
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..100 {
        let tick = world.tick(receiver.drain());
        assert!(!jump_event(&tick, id));
        assert_eq!(world.block_state(DIM, CROP), Some(carrot("7")));
        assert!(horizontal_distance(position(&world, id), START) < 0.1);
    }
    world.tick_rate_mut().set_frozen(true);
    for text in [
        "gamerule mob_griefing false".to_owned(),
        format!("data merge entity {uuid} {{NoAI:0b}}"),
    ] {
        command(&sender, &receiver, &mut world, &text);
    }
    let crate::entity::EntityData::GeneralLiving(data) =
        world.stored_entity_for_test(DIM, id).unwrap().data()
    else {
        panic!("rabbit has living data");
    };
    assert!(!data.is_no_ai());
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..400 {
        world.tick(receiver.drain());
        assert_eq!(world.block_state(DIM, CROP), Some(carrot("7")));
    }
    assert_eq!(cooldown(&world, id), 0);
    world.tick_rate_mut().set_frozen(true);
    for text in [
        "gamerule mob_griefing true".to_owned(),
        "setblock 12 100 8 minecraft:carrots[age=6]".to_owned(),
        format!("tp {uuid} 8.5 100 8.5"),
    ] {
        command(&sender, &receiver, &mut world, &text);
    }
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..600 {
        world.tick(receiver.drain());
        assert_eq!(world.block_state(DIM, CROP), Some(carrot("6")));
    }
    assert_eq!(cooldown(&world, id), 0);
    assert_no_drops(&world);
    world.tick_rate_mut().set_frozen(true);
    for text in [
        "setblock 12 100 8 minecraft:carrots[age=7]".to_owned(),
        format!("tp {uuid} 8.5 100 8.5"),
    ] {
        command(&sender, &receiver, &mut world, &text);
    }
    assert_eq!(world.block_state(DIM, CROP), Some(carrot("7")));
    assert_eq!(position(&world, id), START);
    let mut jumped = false;
    let mut sounded = false;
    let mut airborne = false;
    let mut travelled = 0.0_f64;
    let mut crop_particles = false;
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..4000 {
        let tick = world.tick(receiver.drain());
        jumped |= jump_event(&tick, id);
        sounded |= jump_sound(&tick);
        let at = position(&world, id);
        airborne |= at.y > START.y + 0.1;
        travelled = travelled.max(horizontal_distance(at, START));
        crop_particles |= tick.events.iter().any(|event| matches!(event,
            WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::LevelEvent(event) }
            if event.event_type == 2001 && event.pos == CROP && event.data == carrot("7")
        ));
        assert_no_drops(&world);
        if world.block_state(DIM, CROP) == Some(carrot("6")) {
            break;
        }
    }
    world.tick_rate_mut().set_frozen(true);
    assert_eq!(world.block_state(DIM, CROP), Some(carrot("6")));
    assert!(jumped && sounded && airborne && travelled > 1.0, "real navigation and hopping: event={jumped}, sound={sounded}, airborne={airborne}, travel={travelled}");
    assert!(
        crop_particles,
        "the viewer receives crop particles naming the old ripe state"
    );
    assert!(world
        .game_events(DIM)
        .0
        .iter()
        .any(|event| event.game_event == VanillaGameEvent::BLOCK_CHANGE
            && event.source_entity_id == Some(id)));
    let remaining = cooldown(&world, id);
    assert!(remaining > 0, "eating sets the real saved food cooldown");
    let rabbit = world.stored_entity_for_test(DIM, id).unwrap();
    assert_eq!(rabbit.uuid(), uuid);
    assert_eq!(rabbit.living().unwrap().health(), health);
    let request = world
        .save_request(1_786_000_020_000, 220, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    let candidate = storage.load_candidate().unwrap();
    let mut restored = spawn_pinned_restore(config, &candidate);
    restored.tick_rate_mut().set_frozen(true);
    let restored_id = restored
        .live_entity_id_for_uuid_for_test(DIM, uuid)
        .unwrap();
    let rabbit = restored.stored_entity_for_test(DIM, restored_id).unwrap();
    assert_eq!(rabbit.uuid(), uuid);
    assert_eq!(rabbit.living().unwrap().health(), health);
    assert_eq!(cooldown(&restored, restored_id), remaining);
    assert_eq!(restored.block_state(DIM, CROP), Some(carrot("6")));
    assert_no_drops(&restored);
    for text in [
        "fill 6 100 6 14 102 10 minecraft:water".to_owned(),
        format!("tp {uuid} 8.5 100 8.5"),
    ] {
        command(&sender, &receiver, &mut world, &text);
    }
    let water = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:water",
        &BTreeMap::from([("level".into(), "0".into())]),
    )
    .unwrap();
    assert_eq!(
        world.block_state(DIM, BlockPos { x: 8, y: 100, z: 8 }),
        Some(water)
    );
    assert_eq!(position(&world, id), START);
    let mut submerged = false;
    let mut ascended = false;
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..300 {
        world.tick(receiver.drain());
        let rabbit = world.stored_entity_for_test(DIM, id).unwrap();
        submerged |= rabbit.is_in_water();
        ascended |= rabbit.position().y > START.y + 2.0;
        if submerged && ascended {
            break;
        }
    }
    world.tick_rate_mut().set_frozen(true);
    assert!(
        submerged && ascended,
        "a live rabbit swims upward through deep water: submerged={submerged}, ascended={ascended}"
    );
    let rabbit = world.stored_entity_for_test(DIM, id).unwrap();
    assert_eq!(rabbit.uuid(), uuid);
    assert_eq!(rabbit.living().unwrap().health(), health);
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
