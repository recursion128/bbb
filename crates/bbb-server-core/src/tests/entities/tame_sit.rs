//! A tame animal ordered to sit holds still (`SitWhenOrderedToGoal`), while
//! one on its feet follows its owner (`FollowOwnerGoal`).

use super::feeding::interact_entity_packet;
use super::*;

fn uuid_ints(uuid: Uuid) -> Vec<i32> {
    let bits = uuid.as_u128();
    (0..4)
        .map(|word| (bits >> (96 - 32 * word)) as u32 as i32)
        .collect()
}

/// A wolf `dx` blocks from the session, owned by it and sitting as asked.
pub(super) fn an_owned_wolf(world: &mut ServerWorld, owner: Uuid, dx: f64, sitting: bool) -> i32 {
    let wolf_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WOLF_ID;
    let position = world.publication().players[0].position;
    let (wolf, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        wolf_type,
        Vec3d {
            x: position.x + dx,
            ..position
        },
        crate::entity::EntityData::GeneralLiving(GeneralLivingData::new(wolf_type).unwrap()),
    );
    let bbb_protocol::NbtSummaryValue::Compound(mut entries) = world
        .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, wolf)
        .expect("the wolf has a record")
    else {
        panic!("an entity record is a compound")
    };
    entries.retain(|entry| entry.name != "Owner" && entry.name != "Sitting");
    entries.push(bbb_protocol::NbtSummaryEntry {
        name: "Owner".into(),
        value: bbb_protocol::NbtSummaryValue::IntArray(uuid_ints(owner)),
    });
    entries.push(bbb_protocol::NbtSummaryEntry {
        name: "Sitting".into(),
        value: bbb_protocol::NbtSummaryValue::Byte(i8::from(sitting)),
    });
    world.load_stored_entity_record_for_test(Java26BuiltinDimension::Overworld, wolf, &entries);
    wolf
}

fn wolf_x(world: &ServerWorld, wolf: i32) -> f64 {
    world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, wolf)
        .expect("the wolf")
        .position()
        .x
}

fn sitting(world: &ServerWorld, wolf: i32) -> bool {
    let bbb_protocol::NbtSummaryValue::Compound(entries) = world
        .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, wolf)
        .expect("the wolf has a record")
    else {
        panic!("an entity record is a compound")
    };
    entries.iter().any(|entry| {
        entry.name == "Sitting" && entry.value == bbb_protocol::NbtSummaryValue::Byte(1)
    })
}

#[test]
fn a_sitting_wolf_stays_while_a_standing_one_follows_its_owner() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "owner", 0xfeed);
    let owner = world.publication().players[0].uuid;
    let player_x = world.publication().players[0].position.x;
    let seated = an_owned_wolf(&mut world, owner, 2.0, true);
    let standing = an_owned_wolf(&mut world, owner, -11.0, false);
    let seated_x = wolf_x(&world, seated);

    for _ in 0..200 {
        world.tick(receiver.drain());
    }

    assert!(sitting(&world, seated));
    assert!(
        (wolf_x(&world, seated) - seated_x).abs() < 0.01,
        "the seated wolf held its place"
    );
    assert!(
        (wolf_x(&world, standing) - player_x).abs() < 9.0,
        "the standing wolf came to its owner"
    );

    // The owner's empty hand toggles the order (`Wolf.java:474-479`).
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(seated),
    );
    assert!(!sitting(&world, seated));
}

fn interest_update(tick: &TickResult, wolf: i32, interested: bool) -> bool {
    tick.events.iter().any(|event| {
        matches!(
            event.sent_packet(),
            Some(ClientboundPlayPacket::SetEntityData(data))
                if data.id == wolf && data.values.iter().any(|value|
                    value.data_id == bbb_protocol::entity_synched_data::VANILLA_WOLF_INTERESTED_DATA_ID
                        && value.value == EntityDataValueKind::Boolean(interested))
        )
    })
}

fn wait_for_interest(
    world: &mut ServerWorld,
    receiver: &WorldCommandReceiver,
    wolf: i32,
    interested: bool,
) {
    for _ in 0..120 {
        if interest_update(&world.tick(receiver.drain()), wolf, interested) {
            return;
        }
    }
    panic!("the tracker did not publish wolf interested={interested}");
}

#[test]
fn a_wolf_begs_for_either_hand_but_does_not_skip_the_nearest_empty_handed_player() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "owner", 0xfeed);
    let owner = world.publication().players[0].uuid;
    let position = world.publication().players[0].position;
    let [x, y, z] = [
        position.x.floor() as i32,
        position.y.floor() as i32,
        position.z.floor() as i32,
    ];
    console_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!(
            "fill {} {} {} {} {} {} minecraft:stone",
            x - 4,
            y - 1,
            z - 2,
            x + 4,
            y - 1,
            z + 2
        ),
    );
    console_command(
        &sender,
        &receiver,
        &mut world,
        2,
        &format!(
            "fill {} {y} {} {} {} {} minecraft:air",
            x - 4,
            z - 2,
            x + 4,
            y + 3,
            z + 2
        ),
    );
    let wolf = an_owned_wolf(&mut world, owner, 2.0, true);
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        super::feeding::stack_of("minecraft:bone", 1),
    );
    wait_for_interest(&mut world, &receiver, wolf, true);
    world.set_inventory_slot_for_test(ConnectionId(1), 0, ItemStackSummary::empty());
    wait_for_interest(&mut world, &receiver, wolf, false);
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        40,
        super::feeding::stack_of("minecraft:beef", 1),
    );
    wait_for_interest(&mut world, &receiver, wolf, true);
    world.set_inventory_slot_for_test(ConnectionId(1), 40, ItemStackSummary::empty());
    wait_for_interest(&mut world, &receiver, wolf, false);

    join_loaded(&sender, &receiver, &mut world, 2, "farther", 0xbeef);
    let move_player = |world: &mut ServerWorld, connection, dx| {
        assert!(world.teleport_player(
            ConnectionId(connection),
            crate::world::TeleportTransition {
                new_level: Java26BuiltinDimension::Overworld.into(),
                position: Vec3d {
                    x: position.x + dx,
                    ..position
                },
                y_rot: 0.0,
                x_rot: 0.0,
                relatives_mask: 0,
            },
            &mut Vec::new()
        ));
    };
    move_player(&mut world, 2, -2.0);
    world.set_inventory_slot_for_test(
        ConnectionId(2),
        0,
        super::feeding::stack_of("minecraft:bone", 1),
    );
    for _ in 0..120 {
        assert!(
            !interest_update(&world.tick(receiver.drain()), wolf, true),
            "the nearer empty-handed owner blocks the farther bone holder"
        );
    }
    move_player(&mut world, 1, 14.0);
    wait_for_interest(&mut world, &receiver, wolf, true);
    assert!(sitting(&world, wolf));
}
