use super::melee::{fixture, health, ALICE};
use super::*;
use crate::tests::entities::{console_command, console_command_answer};
use bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SWIMMING_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DOLPHIN_ID;
use bbb_protocol::packets::{AttackEntity, PlayerCommand, PlayerCommandAction};
use bbb_server_types::geometry::Vec3Ops;

const SWIMMER: Vec3d = Vec3d {
    x: 12.5,
    y: 66.0,
    z: 8.5,
};
const GRACE: &str = "minecraft:dolphins_grace";

fn pool(world: &mut ServerWorld, surface_y: i32) {
    let water = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:water",
        &BTreeMap::from([("level".to_owned(), "0".to_owned())]),
    )
    .unwrap();
    let stone = block_state_id("minecraft:stone");
    for x in 0..=15 {
        for z in 0..=15 {
            for y in 65..=surface_y + 1 {
                let wall = x == 0 || x == 15 || z == 0 || z == 15;
                if !wall && y == surface_y + 1 {
                    continue;
                }
                assert!(world.set_block(
                    DIMENSION,
                    BlockPos { x, y, z },
                    if wall { stone } else { water },
                    SET_BLOCK_UPDATE_ALL
                ));
            }
        }
    }
}

fn covered_pool(world: &mut ServerWorld) {
    pool(world, 65);
    for x in 4..=11 {
        for z in 5..=12 {
            for y in 65..=67 {
                if y == 67 || x == 4 || x == 11 || z == 5 || z == 12 {
                    assert!(world.set_block(
                        DIMENSION,
                        BlockPos { x, y, z },
                        block_state_id("minecraft:stone"),
                        SET_BLOCK_UPDATE_ALL
                    ));
                }
            }
        }
    }
}

fn start_swimming(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    connection: u64,
) {
    play(
        sender,
        receiver,
        world,
        connection,
        PlayServerbound::PlayerCommand(PlayerCommand {
            entity_id: world.player_entity_id_for_test(ConnectionId(connection)),
            action: PlayerCommandAction::StartSprinting,
            data: 0,
        }),
    );
    world.tick(Vec::new());
    assert_eq!(
        world.player_pose_id_for_test(ConnectionId(connection)),
        VANILLA_ENTITY_POSE_SWIMMING_ID
    );
}

#[test]
fn dolphin_follows_a_swimmer_refreshes_grace_and_lets_it_expire_on_land() {
    let (sender, receiver, mut world) = fixture();
    pool(&mut world, 69);
    world.set_player_position_for_test(ALICE, SWIMMER);
    start_swimming(&sender, &receiver, &mut world, 1);
    let start = Vec3d { x: 5.5, ..SWIMMER };
    let id = spawn(&mut world, VANILLA_ENTITY_TYPE_DOLPHIN_ID, start).0;
    for _ in 0..150 {
        world.tick(Vec::new());
    }
    let effect = world
        .player_active_effect_for_test(ALICE, GRACE)
        .expect("swimming dolphin refreshes grace");
    assert_eq!(effect.amplifier(), 0);
    assert!(effect.duration() > 0 && effect.duration() <= 100);
    let dolphin = world.stored_entity_for_test(DIMENSION, id).unwrap();
    assert!(
        dolphin.position().distance_to_sqr(SWIMMER) < start.distance_to_sqr(SWIMMER) - 1.0,
        "dolphin swims closer: {:?}",
        dolphin.position()
    );

    world.set_player_position_for_test(ALICE, Vec3d { y: 80.0, ..SWIMMER });
    for _ in 0..10 {
        world.tick(Vec::new());
    }
    assert_ne!(
        world.player_pose_id_for_test(ALICE),
        VANILLA_ENTITY_POSE_SWIMMING_ID
    );
    let remaining = world
        .player_active_effect_for_test(ALICE, GRACE)
        .unwrap()
        .duration();
    for _ in 0..10 {
        world.tick(Vec::new());
    }
    assert_eq!(
        world
            .player_active_effect_for_test(ALICE, GRACE)
            .unwrap()
            .duration(),
        remaining - 10
    );
    for _ in 0..100 {
        world.tick(Vec::new());
    }
    assert!(world.player_active_effect_for_test(ALICE, GRACE).is_none());
}

#[test]
fn dolphin_checks_the_nearest_player_before_looking_for_a_swimmer() {
    let (sender, receiver, mut world) = fixture();
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    pool(&mut world, 69);
    let bob = ConnectionId(2);
    world.set_player_position_for_test(ALICE, Vec3d { x: 7.5, ..SWIMMER });
    world.set_player_position_for_test(bob, SWIMMER);
    start_swimming(&sender, &receiver, &mut world, 2);
    spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_DOLPHIN_ID,
        Vec3d { x: 5.5, ..SWIMMER },
    );
    for _ in 0..10 {
        world.tick(Vec::new());
    }
    assert!(world.player_active_effect_for_test(ALICE, GRACE).is_none());
    assert!(world.player_active_effect_for_test(bob, GRACE).is_none());
    world.set_player_position_for_test(ALICE, Vec3d { y: 90.0, ..SWIMMER });
    tick_until(&mut world, 40, |world| {
        world.player_active_effect_for_test(bob, GRACE).is_some()
    });
}

#[test]
fn an_idle_dolphin_looks_toward_a_nearby_player() {
    let (_sender, _receiver, mut world) = fixture();
    let water = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:water",
        &BTreeMap::from([("level".to_owned(), "0".to_owned())]),
    )
    .unwrap();
    for (x, z) in [(4, 8), (6, 8), (5, 7), (5, 9)] {
        assert!(world.set_block(
            DIMENSION,
            BlockPos { x, y: 65, z },
            block_state_id("minecraft:stone"),
            SET_BLOCK_UPDATE_ALL
        ));
    }
    assert!(world.set_block(
        DIMENSION,
        BlockPos { x: 5, y: 65, z: 8 },
        water,
        SET_BLOCK_UPDATE_ALL
    ));
    let players = [
        Vec3d {
            x: 8.5,
            y: 69.0,
            ..SWIMMER
        },
        Vec3d {
            x: 2.5,
            y: 69.0,
            ..SWIMMER
        },
    ];
    for player in players {
        assert!(world.set_block(
            DIMENSION,
            BlockPos {
                x: player.x as i32,
                y: 68,
                z: 8
            },
            block_state_id("minecraft:stone"),
            SET_BLOCK_UPDATE_ALL
        ));
    }
    world.set_player_position_for_test(ALICE, players[0]);
    let id = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_DOLPHIN_ID,
        Vec3d {
            x: 5.5,
            y: 65.0,
            ..SWIMMER
        },
    )
    .0;
    let mut observed = 0;
    let mut looking_ticks = 0;
    for _ in 0..550 {
        world.tick(Vec::new());
        let dolphin = world.stored_entity_for_test(DIMENSION, id).unwrap();
        let position = dolphin.position();
        let player = players[observed];
        let expected = (player.z - position.z)
            .atan2(player.x - position.x)
            .to_degrees() as f32
            - 90.0
            + 20.0;
        let error = (dolphin.y_head_rot() - expected + 180.0).rem_euclid(360.0) - 180.0;
        looking_ticks = if error.abs() < 3.0 {
            looking_ticks + 1
        } else {
            0
        };
        if looking_ticks >= 5 {
            observed += 1;
            if observed == players.len() {
                break;
            }
            looking_ticks = 0;
            world.set_player_position_for_test(ALICE, players[observed]);
        }
    }
    assert_eq!(
        observed,
        players.len(),
        "the dolphin follows the nearby player's position"
    );
    assert_eq!(health(&world), 20.0, "an idle dolphin remains peaceful");
}

#[test]
fn a_dolphin_retaliates_when_a_survival_player_attacks_it() {
    let (sender, receiver, mut world) = fixture();
    covered_pool(&mut world);
    world.set_player_position_for_test(
        ALICE,
        Vec3d {
            x: 8.5,
            y: 65.0,
            ..SWIMMER
        },
    );
    let (id, uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_DOLPHIN_ID,
        Vec3d {
            x: 6.5,
            y: 65.0,
            ..SWIMMER
        },
    );
    world.tick(Vec::new());
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity { entity_id: id }),
    );
    tick_until(&mut world, 300, |world| health(world) < 20.0);
    assert_eq!(health(&world), 17.0, "dolphin attack damage on Normal");
    let frozen = console_command(&sender, &receiver, &mut world, 20, "tick freeze");
    assert_eq!(console_command_answer(&frozen).0, Some(1));
    let player = snapshot_player(&frozen, 1);
    let player_uuid = player.uuid;
    let player_id = player.entity_id;
    assert_eq!(player_uuid, Uuid::from_u128(1));
    let predicate = "{condition:'minecraft:entity_properties',entity:'this',predicate:{type:'minecraft:dolphin',targeted_entity:{type:'minecraft:player'}}}";
    let positive = format!("execute as {uuid} if predicate {predicate}");
    let inverted = format!(
        "execute as {uuid} if predicate {{condition:'minecraft:inverted',term:{predicate}}}"
    );
    for (step, (age, baby)) in [(0, false), (-24000, true), (0, false)]
        .into_iter()
        .enumerate()
    {
        let operation = 21 + step as u64 * 3;
        if step > 0 {
            let changed = console_command(
                &sender,
                &receiver,
                &mut world,
                operation,
                &format!("data merge entity {uuid} {{Age:{age}}}"),
            );
            let (result, feedback) = console_command_answer(&changed);
            assert_eq!(result, Some(1));
            assert!(feedback
                .iter()
                .any(|key| key == "commands.data.entity.modified"));
        }
        let dolphin = world.stored_entity_for_test(DIMENSION, id).unwrap();
        assert_eq!(dolphin.uuid(), uuid);
        assert_eq!(dolphin.removal_reason(), None);
        assert!(dolphin.living().unwrap().health() > 0.0);
        let EntityData::GeneralLiving(data) = dolphin.data() else {
            panic!("the actual dolphin has living data");
        };
        assert!(!data.is_no_ai());
        assert_eq!(data.is_baby(), baby);
        let NbtSummaryValue::Compound(saved) = world
            .stored_entity_command_data_for_test(DIMENSION, id)
            .unwrap()
        else {
            panic!("the actual dolphin has canonical data");
        };
        assert!(saved
            .iter()
            .any(|entry| entry.name == "Age" && entry.value == NbtSummaryValue::Int(age)));
        assert_eq!(target_of(&world, id), Some(player_uuid));
        assert!(health(&world) > 0.0);
        let matched = console_command(&sender, &receiver, &mut world, operation + 1, &positive);
        assert_eq!(
            console_command_answer(&matched).0,
            if baby { None } else { Some(1) }
        );
        assert_eq!(snapshot_player(&matched, 1).uuid, player_uuid);
        assert_eq!(snapshot_player(&matched, 1).entity_id, player_id);
        let rejected = console_command(&sender, &receiver, &mut world, operation + 2, &inverted);
        assert_eq!(
            console_command_answer(&rejected).0,
            if baby { Some(1) } else { None }
        );
        assert_eq!(target_of(&world, id), Some(player_uuid));
    }
}
