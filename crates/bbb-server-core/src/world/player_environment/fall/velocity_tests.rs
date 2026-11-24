use super::super::tests::{block, world};
use super::*;
use crate::world::player_packets::{apply_movement, MovementOutcome};
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::MovePlayer;

const PLAYER: ConnectionId = ConnectionId(1);

fn landing_probe(
    block_name: &str,
    velocity: f64,
    shift: bool,
    penetration: f64,
) -> (
    ServerWorld,
    MovePlayer,
    crate::world::player_movement::PlayerMoveProbe,
) {
    let mut world = world(GameType::Survival);
    let feet = crate::world::block_position(world.levels[PLAYER].position());
    let support = BlockPos {
        y: feet.y - 1,
        ..feet
    };
    block(&mut world, support, block_name);
    let dimension = world.levels[PLAYER].dimension;
    let state = world.level_by_id(&dimension).block_state(support);
    let top = bbb_server_types::geometry::static_block_collision_boxes(state)
        .unwrap()
        .iter()
        .map(|shape| shape.max_y)
        .reduce(f64::max)
        .unwrap();
    let player = world.levels.player_mut(PLAYER).unwrap();
    {
        let mut position = player.position();
        position.y = f64::from(support.y) + top + 0.2;
        player.set_position(position);
    }
    player.last_good = player.position();
    player.set_on_ground(false);
    player.set_shared_flag(
        bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
        shift,
    );
    player.set_delta_movement(Vec3d {
        x: 2.0,
        y: velocity,
        z: 3.0,
    });
    let movement = MovePlayer {
        position: Some(Vec3d {
            y: f64::from(support.y) + top - penetration,
            ..player.position()
        }),
        rotation: None,
        on_ground: true,
        horizontal_collision: false,
    };
    let probe = world.player_move_probe(PLAYER, &movement).unwrap();
    assert_eq!(probe.collision_response.unwrap().fall_state, Some(state));
    (world, movement, probe)
}

#[test]
fn player_environment_collision_velocity_uses_solved_block_for_accepted_and_refused_moves() {
    for (block_name, factor) in [
        ("minecraft:stone", 0.0),
        ("minecraft:slime_block", 1.0),
        ("minecraft:red_bed", f64::from(0.66_f32)),
    ] {
        for shift in [false, true] {
            for velocity in [-1.25, 0.75] {
                for (penetration, accepted) in [(1.0e-6, true), (0.1, false)] {
                    let (mut world, movement, probe) =
                        landing_probe(block_name, velocity, shift, penetration);
                    let health = world.levels[PLAYER].damage.health;
                    let outcome = apply_movement(
                        world.levels.player_mut(PLAYER).unwrap(),
                        &mut world.scoreboard,
                        movement,
                        probe,
                    );
                    assert_eq!(
                        matches!(outcome, MovementOutcome::Accepted),
                        accepted,
                        "{block_name}/{shift}/{velocity}/{penetration}"
                    );
                    let expected_y = if shift || factor == 0.0 {
                        velocity * 0.0
                    } else if velocity < 0.0 {
                        -velocity * factor
                    } else {
                        velocity
                    };
                    let player = world.levels.player_at(PLAYER);
                    assert_eq!(
                        player.delta_movement(),
                        Vec3d {
                            x: 2.0,
                            y: expected_y,
                            z: 3.0
                        }
                    );
                    assert_eq!(player.damage.health, health);
                    assert_eq!(player.block_contact.fall_distance, 0.0);
                    assert_eq!(
                        player.block_contact.supporting,
                        probe.collision_response.unwrap().supporting
                    );
                    if !accepted {
                        assert_eq!(player.position(), probe.resolved_position.unwrap());
                    }
                }
            }
        }
    }
}

#[test]
fn player_environment_collision_velocity_clips_only_the_blocked_horizontal_axis() {
    for x_axis in [false, true] {
        let mut world = world(GameType::Survival);
        let start = world.levels[PLAYER].position();
        let feet = crate::world::block_position(start);
        let wall = BlockPos {
            x: feet.x + i32::from(x_axis),
            z: feet.z + i32::from(!x_axis),
            ..feet
        };
        for y in [feet.y, feet.y + 1] {
            block(&mut world, BlockPos { y, ..wall }, "minecraft:stone");
        }
        let player = world.levels.player_mut(PLAYER).unwrap();
        player.last_good = start;
        player.set_delta_movement(Vec3d {
            x: 2.0,
            y: -1.25,
            z: 3.0,
        });
        let movement = MovePlayer {
            position: Some(Vec3d {
                x: start.x + f64::from(x_axis),
                z: start.z + f64::from(!x_axis),
                ..start
            }),
            rotation: None,
            on_ground: false,
            horizontal_collision: false,
        };
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
        assert_eq!(
            world.levels[PLAYER].delta_movement(),
            Vec3d {
                x: if x_axis { 0.0 } else { 2.0 },
                y: -1.25,
                z: if x_axis { 3.0 } else { 0.0 },
            }
        );
    }
}

#[test]
fn player_environment_collision_velocity_uses_exact_y_collision_below_the_horizontal_epsilon() {
    let (mut world, mut movement, _) = landing_probe("minecraft:slime_block", -0.5, false, 1.0e-7);
    let target = movement.position.unwrap();
    let player = world.levels.player_mut(PLAYER).unwrap();
    {
        let mut position = player.position();
        position.y = target.y + 2.0e-7;
        player.set_position(position);
    }
    player.last_good = player.position();
    movement.on_ground = true;
    let probe = world.player_move_probe(PLAYER, &movement).unwrap();
    assert!(probe.next_vertical_collision_below);
    assert!(matches!(
        apply_movement(
            world.levels.player_mut(PLAYER).unwrap(),
            &mut world.scoreboard,
            movement,
            probe
        ),
        MovementOutcome::Accepted
    ));
    assert_eq!(world.levels[PLAYER].delta_movement().y, 0.5);
}

#[test]
fn player_environment_collision_velocity_no_physics_and_unclipped_moves_preserve_velocity() {
    for spectator in [false, true] {
        let (mut world, mut movement, _) = landing_probe("minecraft:stone", -0.5, false, 0.1);
        let player = world.levels.player_mut(PLAYER).unwrap();
        let before = player.delta_movement();
        if spectator {
            player.game_type = GameType::Spectator;
            player.movement.vertical_collision_below = true;
            player.block_contact.stuck_multiplier = Vec3d {
                x: 0.25,
                y: 0.05,
                z: 0.25,
            };
        } else {
            movement.position = Some(Vec3d {
                y: player.position().y + 0.1,
                ..player.position()
            });
        }
        let probe = world.player_move_probe(PLAYER, &movement).unwrap();
        assert!(!probe.used_stuck_multiplier);
        assert!(matches!(
            apply_movement(
                world.levels.player_mut(PLAYER).unwrap(),
                &mut world.scoreboard,
                movement,
                probe
            ),
            MovementOutcome::Accepted
        ));
        assert_eq!(world.levels[PLAYER].delta_movement(), before);
        if spectator {
            assert!(!world.levels[PLAYER].movement.vertical_collision_below);
            assert_eq!(
                world.levels[PLAYER].block_contact.stuck_multiplier,
                Vec3d {
                    x: 0.25,
                    y: 0.05,
                    z: 0.25
                }
            );
        }
    }
}

#[test]
fn player_environment_collision_velocity_tiny_clipped_move_keeps_the_old_effect_position() {
    let mut world = world(GameType::Survival);
    let feet = crate::world::block_position(world.levels[PLAYER].position());
    block(
        &mut world,
        BlockPos {
            x: feet.x + 1,
            y: feet.y - 1,
            ..feet
        },
        "minecraft:slime_block",
    );
    let player = world.levels.player_mut(PLAYER).unwrap();
    {
        let mut position = player.position();
        position.x = f64::from(feet.x) + 0.99;
        player.set_position(position);
    }
    {
        let mut position = player.position();
        position.y = f64::from(feet.y) + 0.0001;
        player.set_position(position);
    }
    player.last_good = player.position();
    player.set_on_ground(false);
    player.block_contact.supporting = None;
    player.block_contact.on_ground_no_blocks = false;
    player.set_delta_movement(Vec3d {
        x: 0.0,
        y: -1.25,
        z: 0.0,
    });
    let movement = MovePlayer {
        position: Some(Vec3d {
            y: player.position().y - 0.1,
            ..player.position()
        }),
        rotation: None,
        on_ground: true,
        horizontal_collision: false,
    };
    let probe = world.player_move_probe(PLAYER, &movement).unwrap();
    assert!(probe.contact_movement.is_none());
    assert!(probe.next_vertical_collision_below);
    assert!(probe.collision_response.unwrap().supporting.is_none());
    assert!(matches!(
        apply_movement(
            world.levels.player_mut(PLAYER).unwrap(),
            &mut world.scoreboard,
            movement,
            probe
        ),
        MovementOutcome::MovedWrongly(_)
    ));
    assert_eq!(world.levels[PLAYER].delta_movement().y, 0.0);
}

#[test]
fn player_environment_collision_velocity_packet_correction_clears_the_bounce() {
    for (penetration, accepted) in [(1.0e-6, true), (0.1, false)] {
        let (mut world, movement, _) =
            landing_probe("minecraft:slime_block", -1.25, false, penetration);
        let player = world.levels.player_mut(PLAYER).unwrap();
        player.first_good = player.position();
        let mut events = Vec::new();
        world.apply_play(
            PLAYER,
            bbb_protocol::packets::PlayServerbound::MovePlayer(movement),
            &mut events,
        );
        let corrections: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::PlayerPosition(change),
                    ..
                } => Some(change),
                _ => None,
            })
            .collect();
        if accepted {
            assert!(corrections.is_empty());
            assert_eq!(
                world.levels[PLAYER].delta_movement(),
                Vec3d {
                    x: 2.0,
                    y: 1.25,
                    z: 3.0
                }
            );
        } else {
            assert_eq!(corrections.len(), 1);
            assert_eq!(corrections[0].relatives_mask, 0);
            assert_eq!(corrections[0].delta_movement, Vec3d::default());
            assert_eq!(world.levels[PLAYER].delta_movement(), Vec3d::default());
        }
    }
}

#[test]
fn player_environment_collision_velocity_teleport_preserves_only_relative_axes() {
    use bbb_protocol::packets::{
        PLAYER_RELATIVE_DELTA_X, PLAYER_RELATIVE_DELTA_Y, PLAYER_RELATIVE_DELTA_Z,
    };
    for mask in [
        0,
        PLAYER_RELATIVE_DELTA_Y,
        PLAYER_RELATIVE_DELTA_X | PLAYER_RELATIVE_DELTA_Y | PLAYER_RELATIVE_DELTA_Z,
    ] {
        let mut world = world(GameType::Survival);
        let player = world.levels.player_mut(PLAYER).unwrap();
        player.set_delta_movement(Vec3d {
            x: 2.0,
            y: -1.25,
            z: 3.0,
        });
        let transition = crate::world::TeleportTransition {
            new_level: player.dimension,
            position: player.position(),
            y_rot: player.rotation().y_rot,
            x_rot: player.rotation().x_rot,
            relatives_mask: mask,
        };
        world.teleport_set_position(PLAYER, &transition, &mut Vec::new());
        assert_eq!(
            world.levels[PLAYER].delta_movement(),
            Vec3d {
                x: if mask & PLAYER_RELATIVE_DELTA_X != 0 {
                    2.0
                } else {
                    0.0
                },
                y: if mask & PLAYER_RELATIVE_DELTA_Y != 0 {
                    -1.25
                } else {
                    0.0
                },
                z: if mask & PLAYER_RELATIVE_DELTA_Z != 0 {
                    3.0
                } else {
                    0.0
                },
            }
        );
    }
}
