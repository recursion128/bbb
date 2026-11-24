use super::tests::{block, world};
use super::*;
use crate::world::player_packets::{apply_movement, MovementOutcome};
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID;
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SPRINTING;
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::{MovePlayer, PlayerRotation, Vec3d};

const PLAYER: ConnectionId = ConnectionId(1);

fn setup(floor: &str) -> (ServerWorld, MovePlayer) {
    let mut world = world(GameType::Survival);
    let feet = super::super::block_position(world.levels[PLAYER].position());
    block(
        &mut world,
        BlockPos {
            y: feet.y - 1,
            ..feet
        },
        floor,
    );
    let player = world.levels.player_mut(PLAYER).unwrap();
    player.last_good = player.position();
    player.first_good = player.position();
    player.set_on_ground(true);
    player.set_delta_movement(Vec3d {
        x: 2.0,
        y: -0.5,
        z: 3.0,
    });
    let movement = MovePlayer {
        position: Some(Vec3d {
            y: player.position().y + 0.1,
            ..player.position()
        }),
        rotation: Some(PlayerRotation {
            y_rot: 180.0,
            x_rot: 0.0,
        }),
        on_ground: false,
        horizontal_collision: false,
    };
    (world, movement)
}

#[test]
fn player_jump_velocity_uses_attributes_boost_honey_and_old_sprint_yaw() {
    for floor in ["minecraft:stone", "minecraft:honey_block"] {
        for strength in [0.0, 1.0e-5, 0.625] {
            for boost in [None, Some(1)] {
                for (sprinting, yaw, dx, dz) in [
                    (false, 0.0, 0.0, 0.0),
                    (true, 0.0, 0.0, 0.2),
                    (true, 90.0, -0.2, 0.0),
                ] {
                    let (mut world, movement) = setup(floor);
                    if let Some(amplifier) = boost {
                        assert_eq!(
                            world.add_player_mob_effect(
                                PLAYER,
                                crate::entity::ActiveMobEffect::new(
                                    "minecraft:jump_boost",
                                    100,
                                    amplifier
                                )
                                .unwrap(),
                                &mut Vec::new(),
                            ),
                            Some(true)
                        );
                    }
                    let player = world.levels.player_mut(PLAYER).unwrap();
                    player
                        .attributes
                        .get_instance(VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID)
                        .unwrap()
                        .set_base_value(strength);
                    player.set_y_rot(yaw);
                    player.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SPRINTING, sprinting);
                    player.needs_sync = false;
                    let probe = world.player_move_probe(PLAYER, &movement).unwrap();
                    assert_eq!(
                        probe.jump_factor,
                        Some(if floor.ends_with("honey_block") {
                            0.5
                        } else {
                            1.0
                        })
                    );
                    assert!(matches!(
                        apply_movement(
                            world.levels.player_mut(PLAYER).unwrap(),
                            &mut world.scoreboard,
                            movement,
                            probe
                        ),
                        MovementOutcome::Accepted
                    ));
                    let player = world.levels.player_at(PLAYER);
                    let power = strength as f32 * probe.jump_factor.unwrap()
                        + boost.map_or(0.0, |_| 0.2_f32);
                    let jumps = power > 1.0e-5;
                    assert_eq!(
                        player.delta_movement().y,
                        if jumps { f64::from(power) } else { -0.5 }
                    );
                    assert!(
                        (player.delta_movement().x - (2.0 + if jumps { dx } else { 0.0 })).abs()
                            < 1.0e-7
                    );
                    assert!(
                        (player.delta_movement().z - (3.0 + if jumps { dz } else { 0.0 })).abs()
                            < 1.0e-7
                    );
                    assert_eq!(player.needs_sync, jumps);
                    assert_eq!(player.rotation().y_rot, -180.0);
                    assert_eq!(
                        player
                            .cold
                            .stats
                            .value(bbb_server_types::stats::custom_stat("minecraft:jump")),
                        1
                    );
                    assert_eq!(
                        player.food.exhaustion_level(),
                        if sprinting { 0.2 } else { 0.05 }
                    );
                }
            }
        }
    }
}

#[test]
fn player_jump_velocity_preserves_higher_y_and_does_not_repeat_in_the_air() {
    let (mut world, mut movement) = setup("minecraft:stone");
    {
        let mut delta = world.levels[PLAYER].delta_movement();
        delta.y = 1.5;
        world.levels[PLAYER].set_delta_movement(delta);
    }
    let probe = world.player_move_probe(PLAYER, &movement).unwrap();
    assert!(matches!(
        apply_movement(
            world.levels.player_mut(PLAYER).unwrap(),
            &mut world.scoreboard,
            movement,
            probe
        ),
        MovementOutcome::Accepted
    ));
    assert_eq!(world.levels[PLAYER].delta_movement().y, 1.5);
    assert!(world.levels[PLAYER].needs_sync);
    movement.position.as_mut().unwrap().y += 0.1;
    let probe = world.player_move_probe(PLAYER, &movement).unwrap();
    assert!(probe.jump_factor.is_none());
    assert!(matches!(
        apply_movement(
            world.levels.player_mut(PLAYER).unwrap(),
            &mut world.scoreboard,
            movement,
            probe
        ),
        MovementOutcome::Accepted
    ));
    assert_eq!(
        world.levels[PLAYER]
            .cold
            .stats
            .value(bbb_server_types::stats::custom_stat("minecraft:jump")),
        1
    );
}

#[test]
fn player_jump_velocity_precedes_stuck_and_packet_collision_correction() {
    for stuck in [false, true] {
        let (mut world, mut movement) = setup("minecraft:stone");
        let feet = super::super::block_position(world.levels[PLAYER].position());
        for y in [feet.y, feet.y + 1] {
            block(
                &mut world,
                BlockPos {
                    x: feet.x + 1,
                    y,
                    ..feet
                },
                "minecraft:stone",
            );
        }
        movement.position.as_mut().unwrap().x += 1.0;
        if stuck {
            world.levels[PLAYER].block_contact.stuck_multiplier = Vec3d {
                x: 1.0,
                y: 0.1,
                z: 1.0,
            };
        }
        let probe = world.player_move_probe(PLAYER, &movement).unwrap();
        assert!(matches!(
            apply_movement(
                world.levels.player_mut(PLAYER).unwrap(),
                &mut world.scoreboard,
                movement,
                probe
            ),
            MovementOutcome::MovedWrongly(_)
        ));
        let player = world.levels.player_at(PLAYER);
        assert_eq!(player.delta_movement().x, 0.0);
        assert_eq!(
            player.delta_movement().y,
            if stuck { 0.0 } else { f64::from(0.42_f32) }
        );
        assert!(player.needs_sync);
        assert_eq!(player.food.exhaustion_level(), 0.05);
        let (mut world, _) = setup("minecraft:stone");
        for y in [feet.y, feet.y + 1] {
            block(
                &mut world,
                BlockPos {
                    x: feet.x + 1,
                    y,
                    ..feet
                },
                "minecraft:stone",
            );
        }
        let mut events = Vec::new();
        world.apply_play(
            PLAYER,
            bbb_protocol::packets::PlayServerbound::MovePlayer(movement),
            &mut events,
        );
        assert_eq!(world.levels[PLAYER].delta_movement(), Vec3d::default());
        assert_eq!(world.levels[PLAYER].food.exhaustion_level(), 0.05);
        assert!(events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: super::super::ClientboundPlayPacket::PlayerPosition(_),
                ..
            }
        )));
    }
}

#[test]
fn player_jump_velocity_reads_old_support_with_jump_offset_and_here_precedence() {
    for support in [
        "minecraft:oak_fence",
        "minecraft:cobblestone_wall",
        "minecraft:oak_fence_gate",
    ] {
        let (mut world, movement) = setup("minecraft:stone");
        let feet = super::super::block_position(world.levels[PLAYER].position());
        let supporting = BlockPos {
            x: feet.x + 1,
            y: feet.y - 2,
            ..feet
        };
        block(&mut world, supporting, support);
        block(
            &mut world,
            BlockPos {
                x: supporting.x,
                y: feet.y - 1,
                ..feet
            },
            "minecraft:honey_block",
        );
        world.levels[PLAYER].block_contact.supporting = Some(supporting);
        let probe = world.player_move_probe(PLAYER, &movement).unwrap();
        assert_eq!(
            probe.jump_factor,
            Some(if support.ends_with("fence") { 0.5 } else { 1.0 })
        );
        block(&mut world, feet, "minecraft:honey_block");
        let probe = world.player_move_probe(PLAYER, &movement).unwrap();
        assert_eq!(probe.jump_factor, Some(0.5));
    }
}
