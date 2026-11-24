use super::*;

fn glide(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    position: Vec3d,
    horizontal_collision: bool,
) {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(position),
            rotation: None,
            on_ground: false,
            horizontal_collision,
        }),
    );
}

/// Health after gliding at `speed` blocks per tick along +x into a stone wall.
fn health_after_crash(speed: f64) -> f32 {
    let mut world = ServerWorld::new(ServerWorldConfig {
        game_type: GameType::Survival,
        ..Default::default()
    })
    .unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let start = world.publication().players[0].position;
    let air = Vec3d {
        y: start.y + 3.0,
        ..start
    };
    let wall_x = (air.x + speed).floor() as i32 + 1;
    for dy in 0..2 {
        world.set_block(
            Java26BuiltinDimension::Overworld,
            BlockPos {
                x: wall_x,
                y: air.y.floor() as i32 + dy,
                z: air.z.floor() as i32,
            },
            bbb_protocol::block_state_types::vanilla_block_default_state_id(
                bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID,
            )
            .unwrap(),
            SET_BLOCK_UPDATE_ALL,
        );
    }
    super::player_fall_flying::wear_an_elytra_in_the_air(&sender, &receiver, &mut world, 1);
    glide(&sender, &receiver, &mut world, air, false);
    world.set_player_fall_flying_for_test(ConnectionId(1), true);
    let before = world.player_health_for_test(ConnectionId(1));
    glide(
        &sender,
        &receiver,
        &mut world,
        Vec3d {
            x: air.x - speed,
            ..air
        },
        false,
    );
    glide(&sender, &receiver, &mut world, air, false);
    glide(
        &sender,
        &receiver,
        &mut world,
        Vec3d {
            x: f64::from(wall_x) - 0.3,
            ..air
        },
        true,
    );
    before - world.player_health_for_test(ConnectionId(1))
}

#[test]
fn gliding_fast_into_a_wall_hurts_and_a_slow_bump_does_not() {
    // `(1.5 - 0) * 10 - 3`.
    assert!((health_after_crash(1.5) - 12.0).abs() < 1.0e-3);
    assert_eq!(health_after_crash(0.25), 0.0);
}
