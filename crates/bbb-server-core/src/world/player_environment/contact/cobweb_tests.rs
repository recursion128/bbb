use super::super::tests::{block, world};
use super::*;
use crate::entity::{living::FallImpulseContext, ActiveMobEffect};
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_EFFICIENCY_ID;
use bbb_protocol::packets::MovePlayer;

const PLAYER: ConnectionId = ConnectionId(1);

#[test]
fn player_environment_cobweb_uses_weaving_and_flying_rules_at_rest() {
    for mode in [GameType::Survival, GameType::Creative] {
        for weaving in [false, true] {
            for flying in [false, true] {
                for grace in [-1, 0, 4] {
                    let mut world = world(mode);
                    let pos = crate::world::block_position(world.levels[PLAYER].position());
                    block(&mut world, pos, "minecraft:cobweb");
                    if weaving {
                        assert_eq!(
                            world.add_player_mob_effect(
                                PLAYER,
                                ActiveMobEffect::new("minecraft:weaving", 100, 0).unwrap(),
                                &mut Vec::new(),
                            ),
                            Some(true)
                        );
                    }
                    let player = world.levels.player_mut(PLAYER).unwrap();
                    player.block_contact.old_position = player.position();
                    player.abilities.flying = flying;
                    player.block_contact.fall_distance = 9.0;
                    player.cold.fall_impulse = FallImpulseContext {
                        reset_grace_time: grace,
                        impact_position: Some(player.position()),
                    };
                    player
                        .attributes
                        .get_instance(VANILLA_ATTRIBUTE_MOVEMENT_EFFICIENCY_ID)
                        .unwrap()
                        .set_base_value(1.0);
                    world.tick_player_block_contacts(&mut Vec::new());
                    let player = world.levels.player_at(PLAYER);
                    let expected = if flying {
                        Vec3d::default()
                    } else if weaving {
                        Vec3d {
                            x: 0.5,
                            y: 0.25,
                            z: 0.5,
                        }
                    } else {
                        Vec3d {
                            x: 0.25,
                            y: f64::from(0.05_f32),
                            z: 0.25,
                        }
                    };
                    assert_eq!(player.block_contact.stuck_multiplier, expected);
                    assert_eq!(
                        player.block_contact.fall_distance,
                        if flying { 9.0 } else { 0.0 }
                    );
                    assert_eq!(
                        player.cold.fall_impulse.impact_position.is_some(),
                        grace != 0
                    );
                }
            }
        }
    }
}

#[test]
fn player_environment_cobweb_swept_contact_is_consumed_by_next_real_move() {
    let mut world = world(GameType::Survival);
    let from = world.levels[PLAYER].position();
    let pos = BlockPos {
        x: from.x.floor() as i32 + 1,
        y: from.y as i32,
        z: from.z.floor() as i32,
    };
    block(&mut world, pos, "minecraft:cobweb");
    let player = world.levels.player_mut(PLAYER).unwrap();
    player.last_good = from;
    player.block_contact.old_position = from;
    let movement = MovePlayer {
        position: Some(Vec3d {
            x: from.x + 3.0,
            ..from
        }),
        rotation: None,
        on_ground: false,
        horizontal_collision: false,
    };
    let probe = world.player_move_probe(PLAYER, &movement).unwrap();
    assert!(matches!(
        crate::world::player_packets::apply_movement(
            world.levels.player_mut(PLAYER).unwrap(),
            &mut world.scoreboard,
            movement,
            probe,
        ),
        crate::world::player_packets::MovementOutcome::Accepted
    ));
    world.tick_player_block_contacts(&mut Vec::new());
    let multiplier = Vec3d {
        x: 0.25,
        y: f64::from(0.05_f32),
        z: 0.25,
    };
    assert_eq!(
        world.levels[PLAYER].block_contact.stuck_multiplier,
        multiplier
    );
    let player = world.levels.player_mut(PLAYER).unwrap();
    let start = player.position();
    player.last_good = start;
    player.set_delta_movement(Vec3d {
        x: 2.0,
        y: 3.0,
        z: 4.0,
    });
    let movement = MovePlayer {
        position: Some(Vec3d {
            x: start.x + 0.125,
            y: start.y - 0.125,
            z: start.z + 0.125,
        }),
        rotation: None,
        on_ground: false,
        horizontal_collision: false,
    };
    let probe = world.player_move_probe(PLAYER, &movement).unwrap();
    assert!(probe.used_stuck_multiplier);
    let resolved = probe.contact_movement.unwrap().to;
    assert_eq!(
        resolved,
        Vec3d {
            x: start.x + 0.125 * multiplier.x,
            y: start.y - 0.125 * multiplier.y,
            z: start.z + 0.125 * multiplier.z,
        }
    );
    assert!(matches!(
        crate::world::player_packets::apply_movement(
            world.levels.player_mut(PLAYER).unwrap(),
            &mut world.scoreboard,
            movement,
            probe,
        ),
        crate::world::player_packets::MovementOutcome::Accepted
    ));
    let player = world.levels.player_at(PLAYER);
    assert_eq!(player.block_contact.stuck_multiplier, Vec3d::default());
    assert_eq!(player.delta_movement(), Vec3d::default());
    assert!(
        !world
            .player_move_probe(PLAYER, &movement)
            .unwrap()
            .used_stuck_multiplier
    );
}
