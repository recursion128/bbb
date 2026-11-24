use super::*;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID;
use bbb_protocol::entity_types::{
    vanilla_entity_type_dimensions, vanilla_entity_type_id_for_resource_id,
};

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

struct Case {
    kind: &'static str,
    baby: bool,
    rider: &'static str,
    rider_baby: bool,
    scale: f32,
    yaw: f32,
    size: [f32; 2],
    offset: [f64; 3],
}

fn type_id(kind: &str) -> i32 {
    vanilla_entity_type_id_for_resource_id(&format!("minecraft:{kind}")).unwrap()
}

fn baby(kind: &'static str, factor: f32, offset: [f32; 3]) -> Case {
    Case {
        kind,
        baby: true,
        rider: "chicken",
        rider_baby: false,
        scale: 1.0,
        yaw: 0.0,
        size: vanilla_entity_type_dimensions(type_id(kind))
            .unwrap()
            .map(|coordinate| coordinate * factor),
        offset: offset.map(f64::from),
    }
}

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    operation: &mut u64,
    text: &str,
) -> TickResult {
    *operation += 1;
    let tick = console_command(sender, receiver, world, *operation, text);
    let outcome = admin_outcome(&tick);
    let Ok(AdminResult::Command(report)) = &outcome.result else {
        panic!("{text}: {outcome:?}");
    };
    assert!(report.result.is_some(), "{text}: {outcome:?}");
    tick
}

fn id(world: &ServerWorld, uuid: u128) -> i32 {
    world
        .live_entity_id_for_uuid_for_test(DIM, Uuid::from_u128(uuid))
        .unwrap()
}

fn baby_fields(world: &ServerWorld, uuid: u128) -> [Option<NbtSummaryValue>; 2] {
    let data = world
        .stored_entity_command_data_for_test(DIM, id(world, uuid))
        .unwrap();
    ["Age", "IsBaby"].map(|name| crate::loot_runtime::nbt_field(&data, name).cloned())
}

fn assert_scene(world: &ServerWorld, cases: &[Case]) {
    for (index, case) in cases.iter().enumerate() {
        let uuid = 100 + index as u128 * 2;
        let vehicle = id(world, uuid);
        let rider = id(world, uuid + 1);
        assert_eq!(world.stored_passengers_for_test(DIM, vehicle), &[rider]);
        for (entity_id, kind, is_baby) in [
            (vehicle, case.kind, case.baby),
            (rider, case.rider, case.rider_baby),
        ] {
            let entity = world.stored_entity_for_test(DIM, entity_id).unwrap();
            assert_eq!(entity.entity_type_id(), type_id(kind));
            let crate::entity::EntityData::GeneralLiving(living) = entity.data() else {
                panic!("{kind}: living body");
            };
            assert_eq!(living.is_baby(), is_baby, "{kind}");
            if is_baby {
                let fields = baby_fields(world, entity.uuid().as_u128());
                assert!(
                    matches!(&fields[0], Some(NbtSummaryValue::Int(age)) if *age < 0)
                        || fields[1] == Some(NbtSummaryValue::Byte(1)),
                    "{kind}: {fields:?}"
                );
            }
        }
        let vehicle = world.stored_entity_for_test(DIM, vehicle).unwrap();
        let rider = world.stored_entity_for_test(DIM, rider).unwrap();
        assert_eq!(vehicle.uuid(), Uuid::from_u128(uuid));
        assert_eq!(rider.uuid(), Uuid::from_u128(uuid + 1));
        for (actual, expected) in [vehicle.bb_width(), vehicle.bb_height()]
            .into_iter()
            .zip(case.size)
        {
            assert!((actual - expected).abs() < 1.0e-5, "{} body", case.kind);
        }
        if case.rider_baby {
            assert!((rider.bb_width() - 0.49).abs() < 1.0e-5);
            assert!((rider.bb_height() - 0.99).abs() < 1.0e-5);
        }
        assert_eq!(
            vehicle
                .living()
                .unwrap()
                .attribute_value(VANILLA_ATTRIBUTE_SCALE_ID),
            f64::from(case.scale)
        );
        if case.scale != 1.0 {
            let data = world
                .stored_entity_command_data_for_test(DIM, vehicle.id())
                .unwrap();
            let Some(NbtSummaryValue::List(attributes)) =
                crate::loot_runtime::nbt_field(&data, "attributes")
            else {
                panic!("saved living attributes");
            };
            let scale = attributes
                .iter()
                .find(|attribute| {
                    crate::loot_runtime::nbt_field(attribute, "id")
                        == Some(&NbtSummaryValue::String("minecraft:scale".into()))
                })
                .expect("saved scale attribute");
            assert_eq!(
                crate::loot_runtime::nbt_field(scale, "base"),
                Some(&NbtSummaryValue::Double(f64::from(case.scale).to_bits()))
            );
        }
        let actual = [
            rider.position().x - vehicle.position().x,
            rider.position().y - vehicle.position().y,
            rider.position().z - vehicle.position().z,
        ];
        for (actual, expected) in actual.into_iter().zip(case.offset) {
            assert!(
                (actual - expected).abs() < 1.0e-5,
                "{} passenger offset {actual}, expected {expected}",
                case.kind
            );
        }
    }
}

#[test]
fn baby_attachments_position_real_passenger_trees_and_survive_disk_restore() {
    let height = |kind| vanilla_entity_type_dimensions(type_id(kind)).unwrap()[1];
    let mut turtle = baby("turtle", 0.3, [0.0, height("turtle") * 0.3, -0.075]);
    turtle.scale = 1.5;
    turtle.yaw = 90.0;
    turtle.size = turtle.size.map(|coordinate| coordinate * 1.5);
    turtle.offset = [0.1125, f64::from(height("turtle") * 0.3 * 1.5), 0.0];
    let cases = [
        Case {
            kind: "chicken",
            baby: false,
            rider: "zombie",
            rider_baby: true,
            scale: 1.0,
            yaw: 0.0,
            size: vanilla_entity_type_dimensions(type_id("chicken")).unwrap(),
            offset: [0.0, 0.7 - 0.1875, -0.1],
        },
        Case {
            kind: "chicken",
            baby: true,
            rider: "zombie_villager",
            rider_baby: true,
            scale: 1.0,
            yaw: 0.0,
            size: [0.3, 0.4],
            offset: [0.0, 0.4 - 0.125, 0.0],
        },
        turtle,
        baby("panda", 0.5, [0.0, 0.40625, 0.0]),
        baby("horse", 0.7, [0.0, (height("horse") - 0.125) * 0.7, 0.0]),
        baby(
            "skeleton_horse",
            0.7,
            [0.0, (height("skeleton_horse") - 0.25) * 0.7, 0.0],
        ),
        baby(
            "donkey",
            0.5,
            [0.0, (height("donkey") + 0.03125) * 0.5, -0.3125 * 0.5],
        ),
        baby("llama", 0.5, [0.0, (height("llama") - 0.25) * 0.5, -0.15]),
    ];
    let (sender, receiver) = mailbox();
    let root = std::env::temp_dir().join(format!("bbb-baby-passengers-{}", Uuid::new_v4()));
    let storage = created_fixture_save(&root, "Baby passenger attachments");
    let candidate = storage.load_candidate().unwrap();
    let mut config = datapack_config();
    config.keep_alive_interval_ticks = 30_000;
    let mut world = spawn_pinned_restore(config.clone(), &candidate);
    join_loaded(&sender, &receiver, &mut world, 1, "observer", 1);
    let mut operation = 0;
    for text in [
        "gamerule spawn_mobs false",
        "gamerule advance_time false",
        "time set midnight",
        "weather clear",
        "gamemode spectator observer",
    ] {
        command(&sender, &receiver, &mut world, &mut operation, text);
    }
    let origin = world.publication().players[0].position;
    let mut events = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        let uuid = 100 + index as u128 * 2;
        let age = if case.baby { "Age:-24000," } else { "" };
        let rider_baby = if case.rider_baby { "IsBaby:1b," } else { "" };
        let tick = command(
            &sender,
            &receiver,
            &mut world,
            &mut operation,
            &format!(
                "summon minecraft:{} {} {} {} {{UUID:[I;0,0,0,{uuid}],{age}NoAI:1b,NoGravity:1b,Passengers:[{{id:\"minecraft:{}\",UUID:[I;0,0,0,{}],{rider_baby}NoAI:1b,NoGravity:1b}}]}}",
                case.kind,
                origin.x + index as f64 * 2.0,
                origin.y + 2.0,
                origin.z + 2.0,
                case.rider,
                uuid + 1,
            ),
        );
        assert_eq!(console_command_answer(&tick).0, Some(1));
        events.extend(tick.events);
        for text in [
            format!(
                "data merge entity {} {{PersistenceRequired:1b,Rotation:[{}f,0f],attributes:[{{id:\"minecraft:scale\",base:{}d}}]}}",
                Uuid::from_u128(uuid),
                case.yaw,
                case.scale,
            ),
            format!(
                "data merge entity {} {{PersistenceRequired:1b}}",
                Uuid::from_u128(uuid + 1),
            ),
        ] {
            let tick = command(&sender, &receiver, &mut world, &mut operation, &text);
            events.extend(tick.events);
        }
    }
    for _ in 0..4 {
        let tick = world.tick(receiver.drain());
        events.extend(tick.events);
    }
    assert!(
        events.iter().any(|event| matches!(event,
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::SetPassengers(packet),
            } if packet.vehicle_id == id(&world, 100) && packet.passenger_ids == [id(&world, 101)]
        )),
        "the observer receives the actual chicken jockey relation"
    );
    assert_scene(&world, &cases);
    let saved_babies: Vec<_> = (100..116).map(|uuid| baby_fields(&world, uuid)).collect();
    let request = world
        .save_request(1_786_000_021_000, 221, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    let candidate = storage.load_candidate().unwrap();
    let mut restored = spawn_pinned_restore(config, &candidate);
    for (index, expected) in saved_babies.into_iter().enumerate() {
        assert_eq!(baby_fields(&restored, 100 + index as u128), expected);
    }
    join_loaded(&sender, &receiver, &mut restored, 1, "observer", 1);
    for _ in 0..4 {
        restored.tick(receiver.drain());
    }
    assert_scene(&restored, &cases);
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

/// `Minecart.activateMinecart` (Minecart.java:49-53): a powered activator rail
/// throws the passengers out of a plain minecart.
#[test]
fn a_powered_activator_rail_ejects_minecart_passengers() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "observer", 1);
    let mut operation = 0;
    for text in [
        "setblock 3 63 3 minecraft:redstone_block",
        "setblock 3 64 3 minecraft:activator_rail[powered=true]",
        "summon minecraft:minecart 3.5 64 3.5 {UUID:[I;0,0,0,300],Passengers:[{id:'minecraft:pig',UUID:[I;0,0,0,301],NoAI:1b}]}",
    ] {
        command(&sender, &receiver, &mut world, &mut operation, text);
    }
    world.tick(Vec::new());
    world.tick(Vec::new());
    assert!(world
        .stored_passengers_for_test(DIM, id(&world, 300))
        .is_empty());
}

#[test]
fn a_grown_horse_too_wide_for_its_boat_dismounts_while_a_grown_pig_stays() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "observer", 1);
    let mut operation = 0;
    command(
        &sender,
        &receiver,
        &mut world,
        &mut operation,
        "summon minecraft:oak_boat 3.5 64 3.5 {UUID:[I;0,0,0,400],Passengers:[\
         {id:'minecraft:horse',UUID:[I;0,0,0,401],NoAI:1b,Age:-5},\
         {id:'minecraft:pig',UUID:[I;0,0,0,402],NoAI:1b,Age:-5}]}",
    );
    let boat = id(&world, 400);
    assert_eq!(world.stored_passengers_for_test(DIM, boat).len(), 2);
    for _ in 0..5 {
        world.tick(Vec::new());
    }
    assert_eq!(
        world.stored_passengers_for_test(DIM, boat),
        vec![id(&world, 402)]
    );
    // AbstractBoat.java:630-658: the horse steps off beside the boat rather
    // than onto its top.
    let horse = world
        .stored_entity_for_test(DIM, id(&world, 401))
        .unwrap()
        .position();
    let boat = world.stored_entity_for_test(DIM, boat).unwrap().position();
    assert!(
        (horse.x - boat.x).hypot(horse.z - boat.z) > 1.0,
        "{horse:?} beside {boat:?}"
    );
}

#[test]
fn a_shulker_riding_a_minecart_attaches_to_its_floor() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "observer", 1);
    let mut operation = 0;
    command(
        &sender,
        &receiver,
        &mut world,
        &mut operation,
        "summon minecraft:minecart 3.5 64 3.5 {Passengers:[\
         {id:'minecraft:shulker',UUID:[I;0,0,0,501],AttachFace:1b}]}",
    );
    world.tick(Vec::new());
    let crate::entity::EntityData::GeneralLiving(data) = world
        .stored_entity_for_test(DIM, id(&world, 501))
        .unwrap()
        .data()
    else {
        unreachable!()
    };
    assert_eq!(
        data.shulker_state().unwrap().attach_face(),
        bbb_protocol::packets::Direction::Down
    );
}
