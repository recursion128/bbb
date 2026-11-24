use super::*;

#[test]
fn sleeping_receivers_refuse_pair_push_and_resume_after_waking() {
    let (mut store, ids) = store_of(&[
        (1, VANILLA_ENTITY_TYPE_ZOMBIE_ID, 0.5),
        (2, VANILLA_ENTITY_TYPE_ZOMBIE_ID, 0.6),
    ]);
    let mut body = store.detach(ids[0]).unwrap();
    let mut native = NativeLevel::new();
    store
        .get_mut(ids[1])
        .unwrap()
        .set_sleeping_pos(Some(BlockPos { x: 0, y: 64, z: 0 }));
    assert_eq!(
        native.with_level(|level| push_pair(&mut body, &mut store, level, ids[1])),
        Some(())
    );
    assert_eq!(body.delta_movement(), Vec3d::default());
    assert_eq!(delta_x(&store, ids[1]), 0.0);
    store.get_mut(ids[1]).unwrap().set_sleeping_pos(None);
    assert_eq!(
        native.with_level(|level| push_pair(&mut body, &mut store, level, ids[1])),
        Some(())
    );
    assert!(body.delta_movement().x < 0.0);
    assert!(delta_x(&store, ids[1]) > 0.0);

    let (mut native, sender, receiver) = session_level(0.5, GameType::Survival);
    command(&mut native, &sender, &receiver, "time set night");
    for (z, part) in [(8, "foot"), (7, "head")] {
        let properties = std::collections::BTreeMap::from([
            ("facing".to_owned(), "north".to_owned()),
            ("part".to_owned(), part.to_owned()),
            ("occupied".to_owned(), "false".to_owned()),
        ]);
        let state = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
            "minecraft:red_bed",
            &properties,
        )
        .unwrap();
        native
            .world
            .set_block(DIMENSION, BlockPos { x: 0, y: 64, z }, state, 2 | 16);
    }
    sender
        .try_send(crate::world::WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: bbb_protocol::packets::PlayServerbound::UseItemOn(
                bbb_protocol::packets::UseItemOn {
                    hand: bbb_facts::interaction_hands::InteractionHand::MainHand,
                    hit: bbb_protocol::packets::BlockHitResult {
                        pos: BlockPos { x: 0, y: 64, z: 8 },
                        direction: bbb_protocol::packets::Direction::Up,
                        cursor_x: 0.5,
                        cursor_y: 0.5,
                        cursor_z: 0.5,
                        inside: false,
                        world_border_hit: false,
                    },
                    sequence: 1,
                },
            ),
        })
        .unwrap();
    native.world.tick(receiver.drain());
    assert_eq!(
        native.world.player_pose_id_for_test(ConnectionId(1)),
        bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID
    );
    let sleeping = native.world.nearby_players_for_test(DIMENSION)[0];
    let mob = spawn_world_mob(
        &mut native,
        2,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        Vec3d {
            x: sleeping.position.x + 0.1,
            ..sleeping.position
        },
    );
    native.world.tick(receiver.drain());
    assert_eq!(
        native.world.player_pose_id_for_test(ConnectionId(1)),
        bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID
    );
    sender
        .try_send(crate::world::WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: bbb_protocol::packets::PlayServerbound::PlayerCommand(
                bbb_protocol::packets::PlayerCommand {
                    entity_id: sleeping.entity_id,
                    action: bbb_protocol::packets::PlayerCommandAction::StopSleeping,
                    data: 0,
                },
            ),
        })
        .unwrap();
    native.world.tick(receiver.drain());
    assert_eq!(
        native.world.player_pose_id_for_test(ConnectionId(1)),
        bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_STANDING_ID
    );
    native
        .world
        .set_player_position_for_test(ConnectionId(1), sleeping.position);
    let actor = native
        .world
        .stored_entity_mut_for_test(DIMENSION, mob)
        .unwrap();
    actor.set_pos(Vec3d {
        x: sleeping.position.x + 0.1,
        ..sleeping.position
    });
    actor.set_delta_movement(Vec3d::default());
    native.world.tick(receiver.drain());
    assert!(world_delta(&native, mob).x > 0.0);
    assert!(player_delta(&native).x < 0.0);
}
