use super::*;
use crate::tests::entities::{console_command, console_command_answer};
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_FOX_ID};

fn snbt_uuid(uuid: Uuid) -> String {
    let bits = uuid.as_u128();
    format!(
        "[I;{},{},{},{}]",
        (bits >> 96) as i32,
        (bits >> 64) as i32,
        (bits >> 32) as i32,
        bits as i32
    )
}

fn trusted(world: &ServerWorld, fox: i32) -> Vec<NbtSummaryValue> {
    let Some(NbtSummaryValue::Compound(fields)) =
        world.stored_entity_command_data_for_test(DIMENSION, fox)
    else {
        panic!()
    };
    let NbtSummaryValue::List(values) = &fields
        .iter()
        .find(|field| field.name == "Trusted")
        .unwrap()
        .value
    else {
        panic!()
    };
    values.clone()
}

fn uuid_value(uuid: Uuid) -> NbtSummaryValue {
    let bits = uuid.as_u128();
    NbtSummaryValue::IntArray(vec![
        (bits >> 96) as i32,
        (bits >> 64) as i32,
        (bits >> 32) as i32,
        bits as i32,
    ])
}

#[test]
fn world_data_commands_append_replace_and_remove_fox_trusted_references() {
    let (sender, receiver) = mailbox();
    let mut world =
        spawn_pinned_restore(datapack_config(), &restore_candidate(0, None, Vec::new()));
    install_floor(&mut world);
    let (fox, fox_uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_FOX_ID,
        Vec3d {
            x: 4.5,
            y: 65.0,
            z: 4.5,
        },
    );
    let a = Uuid::from_u128(101);
    let b = Uuid::from_u128(102);
    let c = Uuid::from_u128(103);
    for (index, (tail, expected)) in [
        (
            format!(
                "modify entity {fox_uuid} Trusted append value {}",
                snbt_uuid(a)
            ),
            vec![a],
        ),
        (
            format!(
                "modify entity {fox_uuid} Trusted append value {}",
                snbt_uuid(b)
            ),
            vec![a, b],
        ),
        (
            format!(
                "modify entity {fox_uuid} Trusted append value {}",
                snbt_uuid(c)
            ),
            vec![a, c],
        ),
        (format!("remove entity {fox_uuid} Trusted[0]"), vec![c]),
        (
            format!("modify entity {fox_uuid} Trusted set value []"),
            vec![],
        ),
        (
            format!(
                "merge entity {fox_uuid} {{Trusted:[{},{}]}}",
                snbt_uuid(b),
                snbt_uuid(a)
            ),
            vec![b, a],
        ),
        (format!("remove entity {fox_uuid} Trusted"), vec![]),
    ]
    .into_iter()
    .enumerate()
    {
        let tick = console_command(
            &sender,
            &receiver,
            &mut world,
            index as u64 + 1,
            &format!("data {tail}"),
        );
        assert_eq!(console_command_answer(&tick).0, Some(1), "{tail}");
        assert_eq!(
            trusted(&world, fox),
            expected.into_iter().map(uuid_value).collect::<Vec<_>>(),
            "{tail}"
        );
    }
}

#[test]
fn world_fox_data_trusted_removal_changes_first_resolved_victim_used_by_defend_goal() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig {
        game_type: GameType::Survival,
        ..datapack_config()
    });
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    install_floor(&mut world);
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 4.5,
            y: 65.0,
            z: 4.5,
        },
    );
    // A crouching player is no `Fox.AVOID_PLAYERS` subject (`Fox.java:128`),
    // so the fox stays within the session's ticking chunks.
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerInput(bbb_protocol::packets::PlayerInput {
            shift: true,
            ..Default::default()
        }),
    );
    let (fox, fox_uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_FOX_ID,
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: 4.5,
        },
    );
    let (_, unhurt_uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d {
            x: 6.5,
            y: 65.0,
            z: 6.5,
        },
    );
    let (victim, victim_uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d {
            x: 5.5,
            y: 65.0,
            z: 4.5,
        },
    );
    let tick = console_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!(
            "data merge entity {fox_uuid} {{Trusted:[{},{}]}}",
            snbt_uuid(unhurt_uuid),
            snbt_uuid(victim_uuid)
        ),
    );
    assert_eq!(console_command_answer(&tick).0, Some(1));
    let health = world
        .stored_entity_for_test(DIMENSION, victim)
        .unwrap()
        .living()
        .unwrap()
        .health();
    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::Attack(AttackEntity { entity_id: victim }),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    assert!(
        world
            .stored_entity_for_test(DIMENSION, victim)
            .unwrap()
            .living()
            .unwrap()
            .health()
            < health
    );
    for _ in 0..30 {
        world.tick(Vec::new());
        assert_eq!(target_of(&world, fox), None);
    }
    let tick = console_command(
        &sender,
        &receiver,
        &mut world,
        2,
        &format!("data remove entity {fox_uuid} Trusted[0]"),
    );
    assert_eq!(console_command_answer(&tick).0, Some(1));
    assert_eq!(trusted(&world, fox), vec![uuid_value(victim_uuid)]);
    tick_until(&mut world, 100, |world| {
        target_of(world, fox) == Some(Uuid::from_u128(1))
    });
}
